//! The COM classes of the handler: the class factory and the thumbnail
//! provider (`IInitializeWithStream`, `IThumbnailProvider`).

use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_OUTOFMEMORY, E_POINTER, E_UNEXPECTED, S_FALSE,
};
use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, HBITMAP};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl, IStream};
use windows::Win32::UI::Shell::PropertiesSystem::{IInitializeWithStream, IInitializeWithStream_Impl};
use windows::Win32::UI::Shell::{IThumbnailProvider, IThumbnailProvider_Impl, WTS_ALPHATYPE, WTSAT_ARGB};
use windows::core::{BOOL, Error, GUID, HRESULT, IUnknown, Interface, Ref, Result, implement};

use crate::MAX_FILE_BYTES;

/// The class id of the handler (as [`crate::CLSID_TEXT`]).
pub const CLSID: GUID = GUID::from_u128(0xf1f7156e_acb6_4891_9ac2_7e32e078b242);

/// Runs `f`; a panic gives the error `E_UNEXPECTED`.
fn guarded<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err(E_UNEXPECTED.into()))
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, outer: Ref<IUnknown>, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        guarded(|| {
            if ppv.is_null() || riid.is_null() {
                return Err(E_POINTER.into());
            }
            // SAFETY: `ppv` is not null; COM gives a place for the answer.
            unsafe { *ppv = std::ptr::null_mut() };
            if !outer.is_null() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let provider: IUnknown = Provider::default().into();
            // SAFETY: `riid` and `ppv` are valid pointers (checked above).
            unsafe { provider.query(riid, ppv) }.ok()
        })
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// The handler for one file: Windows gives the stream, then asks for the
/// picture.
#[implement(IInitializeWithStream, IThumbnailProvider)]
#[derive(Default)]
struct Provider {
    bytes: Mutex<Option<Vec<u8>>>,
}

impl IInitializeWithStream_Impl for Provider_Impl {
    fn Initialize(&self, stream: Ref<IStream>, _mode: u32) -> Result<()> {
        guarded(|| {
            let stream = stream.ok()?;
            let mut bytes = Vec::new();
            let mut chunk = vec![0u8; 1 << 16];
            loop {
                let mut read = 0u32;
                // SAFETY: `chunk` has room for `chunk.len()` bytes, and `read`
                // is a valid place for the count.
                let hr = unsafe { stream.Read(chunk.as_mut_ptr().cast(), chunk.len() as u32, Some(&mut read)) };
                hr.ok()?;
                let n = (read as usize).min(chunk.len());
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() > MAX_FILE_BYTES {
                    return Err(E_OUTOFMEMORY.into());
                }
                // S_FALSE: the stream has no more bytes.
                if hr == S_FALSE {
                    break;
                }
            }
            *self.bytes.lock().map_err(|_| Error::from(E_UNEXPECTED))? = Some(bytes);
            Ok(())
        })
    }
}

impl IThumbnailProvider_Impl for Provider_Impl {
    fn GetThumbnail(&self, cx: u32, phbmp: *mut HBITMAP, pdwalpha: *mut WTS_ALPHATYPE) -> Result<()> {
        guarded(|| {
            if phbmp.is_null() || pdwalpha.is_null() {
                return Err(E_POINTER.into());
            }
            let guard = self.bytes.lock().map_err(|_| Error::from(E_UNEXPECTED))?;
            let bytes = guard.as_deref().ok_or_else(|| Error::from(E_UNEXPECTED))?;
            let (w, h, pixels) = crate::bgra_picture(bytes, cx.max(1)).ok_or_else(|| Error::from(E_FAIL))?;
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    // Negative: the rows go from the top.
                    biHeight: -(h as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            // SAFETY: `info` describes a 32-bit bitmap; Windows writes the
            // place of its pixels to `bits`.
            let bitmap = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }?;
            if bits.is_null() {
                return Err(E_FAIL.into());
            }
            // SAFETY: the bitmap has `w * h * 4` bytes at `bits` (32 bits per
            // pixel, no padding), and `pixels` has as many.
            unsafe {
                std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
                *phbmp = bitmap;
                *pdwalpha = WTSAT_ARGB;
            }
            Ok(())
        })
    }
}

/// Gives the class factory of the handler to COM.
///
/// # Safety
///
/// COM calls it with valid pointers for the class id, the interface id and
/// the answer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    catch_unwind(|| {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return E_POINTER;
        }
        // SAFETY: the pointers are not null (checked above); COM makes them valid.
        unsafe {
            *ppv = std::ptr::null_mut();
            if *rclsid != CLSID {
                return CLASS_E_CLASSNOTAVAILABLE;
            }
            let factory: IClassFactory = Factory.into();
            factory.query(riid, ppv)
        }
    })
    .unwrap_or(E_UNEXPECTED)
}

/// The DLL stays loaded while its process runs (the process of thumbnails
/// is short), so COM never has to know if objects are still in use.
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[cfg(test)]
mod tests {
    use windows::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetObjectW, HGDIOBJ};
    use windows::Win32::UI::Shell::SHCreateMemStream;

    use super::*;

    /// As Explorer does: the factory, the provider, the stream, the picture.
    fn thumbnail_of(bytes: &[u8], cx: u32) -> Result<(i32, i32)> {
        let mut factory: *mut c_void = std::ptr::null_mut();
        // SAFETY: valid pointers for the ids and the answer.
        unsafe { DllGetClassObject(&CLSID, &IClassFactory::IID, &mut factory) }.ok()?;
        // SAFETY: the call gave an `IClassFactory`.
        let factory = unsafe { IClassFactory::from_raw(factory) };
        // SAFETY: no outer object.
        let init: IInitializeWithStream = unsafe { factory.CreateInstance(None) }?;
        // SAFETY: a stream of the bytes.
        let stream = unsafe { SHCreateMemStream(Some(bytes)) }.ok_or_else(|| Error::from(E_FAIL))?;
        // SAFETY: a valid stream.
        unsafe { init.Initialize(&stream, 0) }?;
        let provider: IThumbnailProvider = init.cast()?;
        let mut bitmap = HBITMAP::default();
        let mut alpha = WTS_ALPHATYPE::default();
        // SAFETY: valid places for the answers.
        unsafe { provider.GetThumbnail(cx, &mut bitmap, &mut alpha) }?;
        assert_eq!(alpha, WTSAT_ARGB);
        let mut info = BITMAP::default();
        // SAFETY: a valid bitmap and a place of its size.
        let n = unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                size_of::<BITMAP>() as i32,
                Some((&mut info as *mut BITMAP).cast()),
            )
        };
        assert!(n > 0);
        // SAFETY: the bitmap is ours.
        let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
        Ok((info.bmWidth, info.bmHeight))
    }

    fn design_bytes() -> Vec<u8> {
        let d = emb_model::design_from_stitches(
            &emb_model::StitchDesign {
                threads: Vec::new(),
                records: (0..=20)
                    .flat_map(|i| {
                        let y = f64::from(i);
                        (0..=8).map(move |k| emb_model::StitchRecord::stitch(f64::from(k) * 2.0, y))
                    })
                    .collect(),
            },
            "x",
        );
        emb_model::project::to_string(&d).into_bytes()
    }

    #[test]
    fn explorer_gets_a_picture_of_a_design() {
        let (w, h) = thumbnail_of(&design_bytes(), 256).expect("a thumbnail");
        assert!(w.max(h) <= 256 && w.max(h) > 200, "{w}×{h}");
    }

    #[test]
    fn a_file_that_is_not_a_design_gives_an_error() {
        assert!(thumbnail_of(b"this is a text file", 128).is_err());
        assert!(thumbnail_of(b"", 128).is_err());
        // Another class id is not served.
        let mut out: *mut c_void = std::ptr::null_mut();
        let other = GUID::from_u128(1);
        // SAFETY: valid pointers.
        let hr = unsafe { DllGetClassObject(&other, &IClassFactory::IID, &mut out) };
        assert_eq!(hr, CLASS_E_CLASSNOTAVAILABLE);
        assert!(out.is_null());
    }
}
