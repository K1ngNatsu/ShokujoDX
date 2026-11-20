//! The operators of the PostScript reader that do not draw (ADR 0138):
//! the stacks, arithmetic, relations, arrays, strings, dictionaries,
//! types, control, files and resources, as the PostScript language
//! reference (chapter 8) describes them. Operators of a printer that have
//! no effect on the artwork (parameters, caches, output) take their
//! operands and give plausible values.

use emb_geom::math;

use super::filters;
use super::vm::{Frame, MAX_DICT_STACK, Obj, OpFn, PsError, R, Slice, Vm, int, num};

pub(super) const CORE: &[(&str, OpFn)] = &[
    // The operand stack.
    ("pop", pop),
    ("exch", exch),
    ("dup", dup),
    ("copy", copy),
    ("index", index),
    ("roll", roll),
    ("clear", clear),
    ("count", count),
    ("mark", mark),
    ("[", mark),
    ("<<", mark),
    ("cleartomark", cleartomark),
    ("counttomark", counttomark),
    ("]", array_end),
    (">>", dict_end),
    // Arithmetic.
    ("add", add),
    ("sub", sub),
    ("mul", mul),
    ("div", div),
    ("idiv", idiv),
    ("mod", modulo),
    ("abs", abs),
    ("neg", neg),
    ("ceiling", ceiling),
    ("floor", floor),
    ("round", round),
    ("truncate", truncate),
    ("sqrt", sqrt),
    ("atan", atan),
    ("cos", cos),
    ("sin", sin),
    ("exp", exp),
    ("ln", ln),
    ("log", log),
    ("rand", rand),
    ("srand", srand),
    ("rrand", rrand),
    // Relations, logic and bits.
    ("eq", eq),
    ("ne", ne),
    ("gt", gt),
    ("ge", ge),
    ("lt", lt),
    ("le", le),
    ("and", and),
    ("or", or),
    ("xor", xor),
    ("not", not),
    ("bitshift", bitshift),
    // Arrays, strings and dictionaries.
    ("array", array),
    ("packedarray", packedarray),
    ("length", length),
    ("get", get),
    ("put", put),
    ("getinterval", getinterval),
    ("putinterval", putinterval),
    ("aload", aload),
    ("astore", astore),
    ("forall", forall),
    ("string", string),
    ("search", search),
    ("anchorsearch", anchorsearch),
    ("token", token),
    ("dict", dict),
    ("maxlength", maxlength),
    ("begin", begin),
    ("end", end),
    ("def", def),
    ("load", load),
    ("store", store),
    ("undef", undef),
    ("known", known),
    ("where", where_),
    ("currentdict", currentdict),
    ("countdictstack", countdictstack),
    ("dictstack", dictstack),
    ("cleardictstack", cleardictstack),
    ("setpacking", pop),
    ("currentpacking", push_false),
    // Types and attributes.
    ("type", type_),
    ("cvlit", cvlit),
    ("cvx", cvx),
    ("xcheck", xcheck),
    ("executeonly", nothing),
    ("noaccess", nothing),
    ("readonly", nothing),
    ("rcheck", pop_true),
    ("wcheck", pop_true),
    ("cvi", cvi),
    ("cvn", cvn),
    ("cvr", cvr),
    ("cvrs", cvrs),
    ("cvs", cvs),
    // Control.
    ("exec", exec),
    ("if", if_),
    ("ifelse", ifelse),
    ("for", for_),
    ("repeat", repeat),
    ("loop", loop_),
    ("exit", exit),
    ("stop", stop),
    ("stopped", stopped),
    ("countexecstack", countexecstack),
    ("execstack", execstack),
    ("quit", quit),
    ("bind", bind),
    // Memory and the system.
    ("save", save),
    ("restore", restore),
    ("vmstatus", vmstatus),
    ("setglobal", pop),
    ("currentglobal", push_false),
    ("gcheck", pop_false),
    ("vmreclaim", pop),
    ("setvmthreshold", pop),
    ("version", version),
    ("languagelevel", languagelevel),
    ("product", product),
    ("revision", push_one),
    ("serialnumber", push_zero),
    ("realtime", realtime),
    ("usertime", realtime),
    ("=", pop),
    ("==", pop),
    ("print", pop),
    ("stack", nothing),
    ("pstack", nothing),
    ("flush", nothing),
    ("echo", pop),
    ("prompt", nothing),
    ("executive", nothing),
    ("handleerror", nothing),
    ("start", nothing),
    ("internaldict", internaldict),
    ("setuserparams", pop),
    ("currentuserparams", empty_dict),
    ("setsystemparams", pop),
    ("currentsystemparams", empty_dict),
    ("setdevparams", pop2),
    ("currentdevparams", pop_empty_dict),
    ("setcachelimit", pop),
    ("cachestatus", cachestatus),
    ("setcacheparams", cleartomark),
    ("currentcacheparams", currentcacheparams),
    ("setucacheparams", cleartomark),
    ("setobjectformat", pop),
    ("currentobjectformat", push_zero),
    // Files.
    ("currentfile", currentfile),
    ("read", read),
    ("readhexstring", readhexstring),
    ("readstring", readstring),
    ("readline", readline),
    ("bytesavailable", bytesavailable),
    ("flushfile", pop),
    ("closefile", closefile),
    ("resetfile", pop),
    ("status", status),
    ("file", file),
    ("filter", filter),
    ("eexec", eexec),
    ("write", pop2),
    ("writestring", pop2),
    ("writehexstring", pop2),
    ("run", unsupported),
    ("deletefile", unsupported),
    ("renamefile", unsupported),
    // Resources.
    ("findresource", findresource),
    ("defineresource", defineresource),
    ("undefineresource", undefineresource),
    ("resourcestatus", resourcestatus),
    ("resourceforall", resourceforall),
    ("findencoding", findencoding),
];

// ----- Small helpers -----

fn nothing(_: &mut Vm) -> R {
    Ok(())
}

fn unsupported(_: &mut Vm) -> R {
    Err(PsError::Unsupported)
}

fn pop(vm: &mut Vm) -> R {
    vm.pop().map(|_| ())
}

fn pop2(vm: &mut Vm) -> R {
    vm.need(2)?;
    vm.drop_n(2);
    Ok(())
}

fn push_false(vm: &mut Vm) -> R {
    vm.push_bool(false)
}

fn push_zero(vm: &mut Vm) -> R {
    vm.push(Obj::Int(0))
}

fn push_one(vm: &mut Vm) -> R {
    vm.push(Obj::Int(1))
}

fn pop_true(vm: &mut Vm) -> R {
    vm.pop()?;
    vm.push_bool(true)
}

fn pop_false(vm: &mut Vm) -> R {
    vm.pop()?;
    vm.push_bool(false)
}

fn empty_dict(vm: &mut Vm) -> R {
    let d = vm.new_dict(8)?;
    vm.push(Obj::Dict(d))
}

fn pop_empty_dict(vm: &mut Vm) -> R {
    vm.pop()?;
    empty_dict(vm)
}

/// A dictionary operand.
pub(super) fn dict_of(o: Obj) -> R<u32> {
    match o {
        Obj::Dict(d) => Ok(d),
        _ => Err(PsError::TypeCheck),
    }
}

/// A procedure (or any object that `exec` runs) operand.
fn proc_of(o: Obj) -> R<Obj> {
    match o {
        Obj::Arr(_, _) | Obj::Name(_, true) | Obj::Op(_) | Obj::Str(_, true) => Ok(o),
        _ => Err(PsError::TypeCheck),
    }
}

/// A part of a slice: `count` from `start`.
fn sub_slice(s: Slice, start: i64, count: i64) -> R<Slice> {
    if start < 0 || count < 0 || start + count > i64::from(s.len) {
        return Err(PsError::RangeCheck);
    }
    Ok(Slice {
        id: s.id,
        start: s.start + start as u32,
        len: count as u32,
    })
}

// ----- The operand stack -----

fn exch(vm: &mut Vm) -> R {
    vm.need(2)?;
    let n = vm.ops.len();
    vm.ops.swap(n - 1, n - 2);
    Ok(())
}

fn dup(vm: &mut Vm) -> R {
    let o = vm.top(0)?;
    vm.push(o)
}

fn copy(vm: &mut Vm) -> R {
    match vm.top(0)? {
        Obj::Int(n) => {
            let n = usize::try_from(n).map_err(|_| PsError::RangeCheck)?;
            vm.need(n + 1)?;
            vm.pop()?;
            let len = vm.ops.len();
            for i in len - n..len {
                let o = vm.ops[i];
                vm.push(o)?;
            }
            Ok(())
        }
        Obj::Arr(dst, x) => {
            let Obj::Arr(src, _) = vm.top(1)? else {
                return Err(PsError::TypeCheck);
            };
            if src.len > dst.len {
                return Err(PsError::RangeCheck);
            }
            let items = vm.arr(src).to_vec();
            let out = sub_slice(dst, 0, i64::from(src.len))?;
            vm.arr_mut(out).copy_from_slice(&items);
            vm.drop_n(2);
            vm.push(Obj::Arr(out, x))
        }
        Obj::Str(dst, x) => {
            let Obj::Str(src, _) = vm.top(1)? else {
                return Err(PsError::TypeCheck);
            };
            if src.len > dst.len {
                return Err(PsError::RangeCheck);
            }
            let bytes = vm.sbytes(src).to_vec();
            let out = sub_slice(dst, 0, i64::from(src.len))?;
            vm.sbytes_mut(out).copy_from_slice(&bytes);
            vm.drop_n(2);
            vm.push(Obj::Str(out, x))
        }
        Obj::Dict(dst) => {
            let src = dict_of(vm.top(1)?)?;
            let entries = vm.dicts[src as usize].entries.clone();
            for (k, v) in entries {
                vm.dict_put(dst, k, v)?;
            }
            vm.drop_n(2);
            vm.push(Obj::Dict(dst))
        }
        // Another copy of a graphics state.
        Obj::GState(dst) => {
            let Obj::GState(src) = vm.top(1)? else {
                return Err(PsError::TypeCheck);
            };
            vm.gstates[dst as usize] = vm.gstates[src as usize].clone();
            vm.drop_n(2);
            vm.push(Obj::GState(dst))
        }
        _ => Err(PsError::TypeCheck),
    }
}

fn index(vm: &mut Vm) -> R {
    let n = usize::try_from(vm.int_at(0)?).map_err(|_| PsError::RangeCheck)?;
    let o = vm.top(n + 1).map_err(|_| PsError::RangeCheck)?;
    vm.pop()?;
    vm.push(o)
}

fn roll(vm: &mut Vm) -> R {
    let j = vm.int_at(0)?;
    let n = usize::try_from(vm.int_at(1)?).map_err(|_| PsError::RangeCheck)?;
    vm.need(n + 2)?;
    vm.drop_n(2);
    if n == 0 {
        return Ok(());
    }
    let len = vm.ops.len();
    let part = &mut vm.ops[len - n..];
    let k = j.rem_euclid(n as i64) as usize;
    part.rotate_right(k);
    Ok(())
}

fn clear(vm: &mut Vm) -> R {
    vm.ops.clear();
    Ok(())
}

fn count(vm: &mut Vm) -> R {
    let n = vm.ops.len() as i64;
    vm.push_int(n)
}

fn mark(vm: &mut Vm) -> R {
    vm.push(Obj::Mark)
}

pub(super) fn cleartomark(vm: &mut Vm) -> R {
    let n = vm.count_to_mark()?;
    vm.drop_n(n + 1);
    Ok(())
}

fn counttomark(vm: &mut Vm) -> R {
    let n = vm.count_to_mark()? as i64;
    vm.push_int(n)
}

fn array_end(vm: &mut Vm) -> R {
    let n = vm.count_to_mark()?;
    let len = vm.ops.len();
    let items = vm.ops[len - n..].to_vec();
    vm.drop_n(n + 1);
    let a = vm.new_array(items)?;
    vm.push(Obj::Arr(a, false))
}

fn dict_end(vm: &mut Vm) -> R {
    let n = vm.count_to_mark()?;
    if n % 2 != 0 {
        return Err(PsError::RangeCheck);
    }
    let len = vm.ops.len();
    let items = vm.ops[len - n..].to_vec();
    let d = vm.new_dict(n / 2)?;
    for pair in items.chunks(2) {
        vm.dict_put(d, pair[0], pair[1])?;
    }
    vm.drop_n(n + 1);
    vm.push(Obj::Dict(d))
}

// ----- Arithmetic -----

fn arith(vm: &mut Vm, i: fn(i64, i64) -> Option<i64>, r: fn(f64, f64) -> f64) -> R {
    let (a, b) = (vm.top(1)?, vm.top(0)?);
    let (x, y) = (num(a)?, num(b)?);
    vm.drop_n(2);
    match (a, b) {
        (Obj::Int(p), Obj::Int(q)) => match i(i64::from(p), i64::from(q)) {
            Some(v) => vm.push_int(v),
            None => vm.push_real(r(x, y)),
        },
        _ => vm.push_real(r(x, y)),
    }
}

fn add(vm: &mut Vm) -> R {
    arith(vm, |a, b| Some(a + b), |a, b| a + b)
}

fn sub(vm: &mut Vm) -> R {
    arith(vm, |a, b| Some(a - b), |a, b| a - b)
}

fn mul(vm: &mut Vm) -> R {
    arith(vm, |a, b| a.checked_mul(b), |a, b| a * b)
}

fn div(vm: &mut Vm) -> R {
    let [a, b] = vm.nums::<2>()?;
    if b == 0.0 {
        return Err(PsError::UndefinedResult);
    }
    vm.push_real(a / b)
}

fn ints2(vm: &mut Vm) -> R<(i64, i64)> {
    let (a, b) = (vm.int_at(1)?, vm.int_at(0)?);
    vm.drop_n(2);
    Ok((a, b))
}

fn idiv(vm: &mut Vm) -> R {
    let (a, b) = ints2(vm)?;
    if b == 0 {
        return Err(PsError::UndefinedResult);
    }
    vm.push_int(a / b)
}

fn modulo(vm: &mut Vm) -> R {
    let (a, b) = ints2(vm)?;
    if b == 0 {
        return Err(PsError::UndefinedResult);
    }
    vm.push_int(a % b)
}

fn unary(vm: &mut Vm, i: fn(i64) -> i64, r: fn(f64) -> f64) -> R {
    match vm.top(0)? {
        Obj::Int(v) => {
            vm.pop()?;
            vm.push_int(i(i64::from(v)))
        }
        Obj::Real(v) => {
            vm.pop()?;
            vm.push_real(r(v))
        }
        _ => Err(PsError::TypeCheck),
    }
}

fn abs(vm: &mut Vm) -> R {
    unary(vm, i64::abs, f64::abs)
}

fn neg(vm: &mut Vm) -> R {
    unary(vm, |v| -v, |v| -v)
}

fn ceiling(vm: &mut Vm) -> R {
    unary(vm, |v| v, f64::ceil)
}

fn floor(vm: &mut Vm) -> R {
    unary(vm, |v| v, f64::floor)
}

fn round(vm: &mut Vm) -> R {
    // A half goes up (PostScript: -1.5 round is -1).
    unary(vm, |v| v, |v| (v + 0.5).floor())
}

fn truncate(vm: &mut Vm) -> R {
    unary(vm, |v| v, f64::trunc)
}

fn sqrt(vm: &mut Vm) -> R {
    let [v] = vm.nums::<1>()?;
    if v < 0.0 {
        return Err(PsError::RangeCheck);
    }
    vm.push_real(v.sqrt())
}

fn atan(vm: &mut Vm) -> R {
    let [y, x] = vm.nums::<2>()?;
    if x == 0.0 && y == 0.0 {
        return Err(PsError::UndefinedResult);
    }
    let d = math::to_degrees(math::atan2(y, x));
    vm.push_real(if d < 0.0 { d + 360.0 } else { d })
}

fn cos(vm: &mut Vm) -> R {
    let [d] = vm.nums::<1>()?;
    vm.push_real(math::cos(math::to_radians(d)))
}

fn sin(vm: &mut Vm) -> R {
    let [d] = vm.nums::<1>()?;
    vm.push_real(math::sin(math::to_radians(d)))
}

fn exp(vm: &mut Vm) -> R {
    let [b, e] = vm.nums::<2>()?;
    vm.push_real(math::pow(b, e))
}

fn ln(vm: &mut Vm) -> R {
    let [v] = vm.nums::<1>()?;
    if v <= 0.0 {
        return Err(PsError::RangeCheck);
    }
    vm.push_real(math::ln(v))
}

fn log(vm: &mut Vm) -> R {
    let [v] = vm.nums::<1>()?;
    if v <= 0.0 {
        return Err(PsError::RangeCheck);
    }
    vm.push_real(math::ln(v) / math::ln(10.0))
}

fn rand(vm: &mut Vm) -> R {
    // A fixed generator, so each import gives the same result.
    vm.seed = vm.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    let v = (vm.seed >> 1) as i64;
    vm.push_int(v)
}

fn srand(vm: &mut Vm) -> R {
    vm.seed = vm.int_at(0)? as u32;
    vm.pop().map(|_| ())
}

fn rrand(vm: &mut Vm) -> R {
    let s = vm.seed as i64;
    vm.push_int(s)
}

// ----- Relations, logic and bits -----

/// `eq` of two objects.
pub(super) fn equal(vm: &Vm, a: Obj, b: Obj) -> bool {
    match (a, b) {
        (Obj::Int(_) | Obj::Real(_), Obj::Int(_) | Obj::Real(_)) => num(a).ok() == num(b).ok(),
        (Obj::Str(x, _), Obj::Str(y, _)) => vm.sbytes(x) == vm.sbytes(y),
        (Obj::Name(x, _), Obj::Name(y, _)) => x == y,
        (Obj::Str(s, _), Obj::Name(n, _)) | (Obj::Name(n, _), Obj::Str(s, _)) => vm.sbytes(s) == vm.name_bytes(n),
        (Obj::Arr(x, _), Obj::Arr(y, _)) => x == y,
        (Obj::File(x, _), Obj::File(y, _)) => x == y,
        _ => a == b,
    }
}

fn eq(vm: &mut Vm) -> R {
    let (a, b) = (vm.top(1)?, vm.top(0)?);
    vm.drop_n(2);
    let v = equal(vm, a, b);
    vm.push_bool(v)
}

fn ne(vm: &mut Vm) -> R {
    let (a, b) = (vm.top(1)?, vm.top(0)?);
    vm.drop_n(2);
    let v = !equal(vm, a, b);
    vm.push_bool(v)
}

fn compare(vm: &mut Vm, test: fn(std::cmp::Ordering) -> bool) -> R {
    let (a, b) = (vm.top(1)?, vm.top(0)?);
    let ord = match (a, b) {
        (Obj::Str(x, _), Obj::Str(y, _)) => vm.sbytes(x).cmp(vm.sbytes(y)),
        _ => num(a)?.partial_cmp(&num(b)?).ok_or(PsError::UndefinedResult)?,
    };
    vm.drop_n(2);
    vm.push_bool(test(ord))
}

fn gt(vm: &mut Vm) -> R {
    compare(vm, |o| o.is_gt())
}

fn ge(vm: &mut Vm) -> R {
    compare(vm, |o| o.is_ge())
}

fn lt(vm: &mut Vm) -> R {
    compare(vm, |o| o.is_lt())
}

fn le(vm: &mut Vm) -> R {
    compare(vm, |o| o.is_le())
}

fn logic(vm: &mut Vm, b: fn(bool, bool) -> bool, i: fn(i64, i64) -> i64) -> R {
    match (vm.top(1)?, vm.top(0)?) {
        (Obj::Bool(x), Obj::Bool(y)) => {
            vm.drop_n(2);
            vm.push_bool(b(x, y))
        }
        (Obj::Int(x), Obj::Int(y)) => {
            vm.drop_n(2);
            vm.push_int(i(i64::from(x), i64::from(y)))
        }
        _ => Err(PsError::TypeCheck),
    }
}

fn and(vm: &mut Vm) -> R {
    logic(vm, |a, b| a && b, |a, b| a & b)
}

fn or(vm: &mut Vm) -> R {
    logic(vm, |a, b| a || b, |a, b| a | b)
}

fn xor(vm: &mut Vm) -> R {
    logic(vm, |a, b| a ^ b, |a, b| a ^ b)
}

fn not(vm: &mut Vm) -> R {
    match vm.top(0)? {
        Obj::Bool(b) => {
            vm.pop()?;
            vm.push_bool(!b)
        }
        Obj::Int(i) => {
            vm.pop()?;
            vm.push(Obj::Int(!i))
        }
        _ => Err(PsError::TypeCheck),
    }
}

fn bitshift(vm: &mut Vm) -> R {
    let (v, s) = ints2(vm)?;
    let v = v as i32 as u32;
    let r = if s >= 32 || s <= -32 {
        0
    } else if s >= 0 {
        v << s
    } else {
        v >> (-s)
    };
    vm.push(Obj::Int(r as i32))
}

// ----- Arrays, strings and dictionaries -----

fn array(vm: &mut Vm) -> R {
    let n = vm.int_at(0)?;
    if !(0..=1 << 24).contains(&n) {
        return Err(PsError::RangeCheck);
    }
    vm.pop()?;
    let a = vm.new_array(vec![Obj::Null; n as usize])?;
    vm.push(Obj::Arr(a, false))
}

fn packedarray(vm: &mut Vm) -> R {
    let n = usize::try_from(vm.int_at(0)?).map_err(|_| PsError::RangeCheck)?;
    vm.need(n + 1)?;
    vm.pop()?;
    let len = vm.ops.len();
    let items = vm.ops[len - n..].to_vec();
    vm.drop_n(n);
    let a = vm.new_array(items)?;
    vm.push(Obj::Arr(a, false))
}

fn length(vm: &mut Vm) -> R {
    let n = match vm.top(0)? {
        Obj::Arr(a, _) => a.len as usize,
        Obj::Str(s, _) => s.len as usize,
        Obj::Dict(d) => vm.dicts[d as usize].entries.len(),
        Obj::Name(n, _) => vm.name_bytes(n).len(),
        _ => return Err(PsError::TypeCheck),
    };
    vm.pop()?;
    vm.push_int(n as i64)
}

fn get(vm: &mut Vm) -> R {
    let (c, k) = (vm.top(1)?, vm.top(0)?);
    let v = match c {
        Obj::Arr(a, _) => {
            let i = int(k)?;
            if i < 0 || i >= i64::from(a.len) {
                return Err(PsError::RangeCheck);
            }
            vm.arr(a)[i as usize]
        }
        Obj::Str(s, _) => {
            let i = int(k)?;
            if i < 0 || i >= i64::from(s.len) {
                return Err(PsError::RangeCheck);
            }
            Obj::Int(i32::from(vm.sbytes(s)[i as usize]))
        }
        Obj::Dict(d) => match vm.dict_get(d, k)? {
            Some(v) => v,
            None => {
                let name = match k {
                    Obj::Name(n, _) => n,
                    _ => vm.intern(b"key"),
                };
                return Err(PsError::Undefined(name));
            }
        },
        _ => return Err(PsError::TypeCheck),
    };
    vm.drop_n(2);
    vm.push(v)
}

fn put(vm: &mut Vm) -> R {
    let (c, k, v) = (vm.top(2)?, vm.top(1)?, vm.top(0)?);
    match c {
        Obj::Arr(a, _) => {
            let i = int(k)?;
            if i < 0 || i >= i64::from(a.len) {
                return Err(PsError::RangeCheck);
            }
            vm.arr_mut(a)[i as usize] = v;
        }
        Obj::Str(s, _) => {
            let i = int(k)?;
            if i < 0 || i >= i64::from(s.len) {
                return Err(PsError::RangeCheck);
            }
            vm.sbytes_mut(s)[i as usize] = (int(v)? & 0xff) as u8;
        }
        Obj::Dict(d) => vm.dict_put(d, k, v)?,
        _ => return Err(PsError::TypeCheck),
    }
    vm.drop_n(3);
    Ok(())
}

fn getinterval(vm: &mut Vm) -> R {
    let (c, i, n) = (vm.top(2)?, vm.int_at(1)?, vm.int_at(0)?);
    let o = match c {
        Obj::Arr(a, x) => Obj::Arr(sub_slice(a, i, n)?, x),
        Obj::Str(s, x) => Obj::Str(sub_slice(s, i, n)?, x),
        _ => return Err(PsError::TypeCheck),
    };
    vm.drop_n(3);
    vm.push(o)
}

fn putinterval(vm: &mut Vm) -> R {
    let (c, i, src) = (vm.top(2)?, vm.int_at(1)?, vm.top(0)?);
    match (c, src) {
        (Obj::Arr(a, _), Obj::Arr(b, _)) => {
            let items = vm.arr(b).to_vec();
            let part = sub_slice(a, i, items.len() as i64)?;
            vm.arr_mut(part).copy_from_slice(&items);
        }
        (Obj::Str(a, _), Obj::Str(b, _)) => {
            let bytes = vm.sbytes(b).to_vec();
            let part = sub_slice(a, i, bytes.len() as i64)?;
            vm.sbytes_mut(part).copy_from_slice(&bytes);
        }
        _ => return Err(PsError::TypeCheck),
    }
    vm.drop_n(3);
    Ok(())
}

fn aload(vm: &mut Vm) -> R {
    let Obj::Arr(a, x) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    vm.pop()?;
    let items = vm.arr(a).to_vec();
    for o in items {
        vm.push(o)?;
    }
    vm.push(Obj::Arr(a, x))
}

fn astore(vm: &mut Vm) -> R {
    let Obj::Arr(a, x) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    let n = a.len as usize;
    vm.need(n + 1)?;
    vm.pop()?;
    let len = vm.ops.len();
    let items = vm.ops[len - n..].to_vec();
    vm.drop_n(n);
    vm.arr_mut(a).copy_from_slice(&items);
    vm.push(Obj::Arr(a, x))
}

fn forall(vm: &mut Vm) -> R {
    let (c, p) = (vm.top(1)?, proc_of(vm.top(0)?)?);
    let frame = match c {
        Obj::Arr(a, _) => Frame::ForallArr { proc: p, arr: a, i: 0 },
        Obj::Str(s, _) => Frame::ForallStr { proc: p, s, i: 0 },
        Obj::Dict(d) => Frame::ForallDict { proc: p, dict: d, i: 0 },
        _ => return Err(PsError::TypeCheck),
    };
    vm.drop_n(2);
    vm.push_frame(frame)
}

fn string(vm: &mut Vm) -> R {
    let n = vm.int_at(0)?;
    if !(0..=1 << 26).contains(&n) {
        return Err(PsError::RangeCheck);
    }
    vm.pop()?;
    let s = vm.new_string(vec![0; n as usize])?;
    vm.push(Obj::Str(s, false))
}

fn two_strings(vm: &Vm) -> R<(Slice, bool, Slice)> {
    match (vm.top(1)?, vm.top(0)?) {
        (Obj::Str(s, x), Obj::Str(k, _)) => Ok((s, x, k)),
        _ => Err(PsError::TypeCheck),
    }
}

fn search(vm: &mut Vm) -> R {
    let (s, x, k) = two_strings(vm)?;
    let (hay, needle) = (vm.sbytes(s), vm.sbytes(k));
    let found = if needle.is_empty() {
        Some(0)
    } else {
        hay.windows(needle.len()).position(|w| w == needle)
    };
    vm.drop_n(2);
    match found {
        Some(i) => {
            let (i, m) = (i as i64, i64::from(k.len));
            let post = sub_slice(s, i + m, i64::from(s.len) - i - m)?;
            vm.push(Obj::Str(post, x))?;
            vm.push(Obj::Str(sub_slice(s, i, m)?, x))?;
            vm.push(Obj::Str(sub_slice(s, 0, i)?, x))?;
            vm.push_bool(true)
        }
        None => {
            vm.push(Obj::Str(s, x))?;
            vm.push_bool(false)
        }
    }
}

fn anchorsearch(vm: &mut Vm) -> R {
    let (s, x, k) = two_strings(vm)?;
    let found = vm.sbytes(s).starts_with(vm.sbytes(k));
    vm.drop_n(2);
    if found {
        let m = i64::from(k.len);
        vm.push(Obj::Str(sub_slice(s, m, i64::from(s.len) - m)?, x))?;
        vm.push(Obj::Str(sub_slice(s, 0, m)?, x))?;
        vm.push_bool(true)
    } else {
        vm.push(Obj::Str(s, x))?;
        vm.push_bool(false)
    }
}

fn token(vm: &mut Vm) -> R {
    match vm.top(0)? {
        Obj::Str(s, x) => {
            vm.pop()?;
            let bytes = vm.sbytes(s).to_vec();
            let src = vm.sources.len() as u32;
            vm.sources.push(std::rc::Rc::from(bytes));
            let mut p = 0;
            let r = vm.scan(src, &mut p);
            vm.sources.pop();
            match r? {
                Some(o) => {
                    let p = (p as i64).min(i64::from(s.len));
                    vm.push(Obj::Str(sub_slice(s, p, i64::from(s.len) - p)?, x))?;
                    vm.push(o)?;
                    vm.push_bool(true)
                }
                None => vm.push_bool(false),
            }
        }
        Obj::File(0, _) => {
            vm.pop()?;
            let Some((frame, _, pos)) = vm.text_rest() else {
                return vm.push_bool(false);
            };
            let Frame::Tokens { src, .. } = vm.exec[frame] else {
                return vm.push_bool(false);
            };
            let mut p = pos;
            let r = vm.scan(src, &mut p);
            vm.set_text_pos(frame, p);
            match r? {
                Some(o) => {
                    vm.push(o)?;
                    vm.push_bool(true)
                }
                None => vm.push_bool(false),
            }
        }
        _ => Err(PsError::TypeCheck),
    }
}

fn dict(vm: &mut Vm) -> R {
    let n = vm.int_at(0)?;
    if n < 0 {
        return Err(PsError::RangeCheck);
    }
    vm.pop()?;
    let d = vm.new_dict(n as usize)?;
    vm.push(Obj::Dict(d))
}

fn maxlength(vm: &mut Vm) -> R {
    let d = dict_of(vm.top(0)?)?;
    vm.pop()?;
    let data = &vm.dicts[d as usize];
    let n = data.cap.max(data.entries.len()) as i64;
    vm.push_int(n)
}

fn begin(vm: &mut Vm) -> R {
    let d = dict_of(vm.top(0)?)?;
    if vm.dstack.len() >= MAX_DICT_STACK {
        return Err(PsError::LimitCheck);
    }
    vm.pop()?;
    vm.dstack.push(d);
    Ok(())
}

/// The dictionaries at the bottom of the stack that `end` does not remove.
fn base_dicts(vm: &Vm) -> usize {
    if vm.ai_script { 4 } else { 3 }
}

fn end(vm: &mut Vm) -> R {
    if vm.dstack.len() <= base_dicts(vm) {
        return Err(PsError::LimitCheck);
    }
    vm.dstack.pop();
    Ok(())
}

fn def(vm: &mut Vm) -> R {
    let (k, v) = (vm.top(1)?, vm.top(0)?);
    let d = vm.current_dict();
    vm.dict_put(d, k, v)?;
    vm.drop_n(2);
    Ok(())
}

fn load(vm: &mut Vm) -> R {
    let k = vm.top(0)?;
    let Some(d) = vm.where_key(k)? else {
        return Err(match k {
            Obj::Name(n, _) => PsError::Undefined(n),
            _ => PsError::TypeCheck,
        });
    };
    let v = vm.dict_get(d, k)?.unwrap_or(Obj::Null);
    vm.pop()?;
    vm.push(v)
}

fn store(vm: &mut Vm) -> R {
    let (k, v) = (vm.top(1)?, vm.top(0)?);
    let d = match vm.where_key(k)? {
        Some(d) => d,
        None => vm.current_dict(),
    };
    vm.dict_put(d, k, v)?;
    vm.drop_n(2);
    Ok(())
}

fn undef(vm: &mut Vm) -> R {
    let (d, k) = (dict_of(vm.top(1)?)?, vm.top(0)?);
    vm.dict_undef(d, k)?;
    vm.drop_n(2);
    Ok(())
}

fn known(vm: &mut Vm) -> R {
    let (d, k) = (dict_of(vm.top(1)?)?, vm.top(0)?);
    let v = vm.dict_get(d, k)?.is_some();
    vm.drop_n(2);
    vm.push_bool(v)
}

fn where_(vm: &mut Vm) -> R {
    let k = vm.top(0)?;
    let found = vm.where_key(k)?;
    vm.pop()?;
    match found {
        Some(d) => {
            vm.push(Obj::Dict(d))?;
            vm.push_bool(true)
        }
        None => vm.push_bool(false),
    }
}

fn currentdict(vm: &mut Vm) -> R {
    let d = vm.current_dict();
    vm.push(Obj::Dict(d))
}

fn countdictstack(vm: &mut Vm) -> R {
    let n = vm.dstack.len() as i64;
    vm.push_int(n)
}

fn dictstack(vm: &mut Vm) -> R {
    let Obj::Arr(a, x) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    let n = vm.dstack.len();
    if n > a.len as usize {
        return Err(PsError::RangeCheck);
    }
    let items: Vec<Obj> = vm.dstack.iter().map(|&d| Obj::Dict(d)).collect();
    let out = sub_slice(a, 0, n as i64)?;
    vm.arr_mut(out).copy_from_slice(&items);
    vm.pop()?;
    vm.push(Obj::Arr(out, x))
}

fn cleardictstack(vm: &mut Vm) -> R {
    let n = base_dicts(vm);
    vm.dstack.truncate(n);
    Ok(())
}

// ----- Types and attributes -----

fn type_(vm: &mut Vm) -> R {
    let t = match vm.top(0)? {
        Obj::Null => "nulltype",
        Obj::Bool(_) => "booleantype",
        Obj::Int(_) => "integertype",
        Obj::Real(_) => "realtype",
        Obj::Name(..) => "nametype",
        Obj::Str(..) => "stringtype",
        Obj::Arr(..) => "arraytype",
        Obj::Dict(_) => "dicttype",
        Obj::Op(_) => "operatortype",
        Obj::Mark => "marktype",
        Obj::File(..) => "filetype",
        Obj::Save(_) => "savetype",
        Obj::Font(_) => "fonttype",
        Obj::GState(_) => "gstatetype",
    };
    vm.pop()?;
    let n = vm.intern(t.as_bytes());
    vm.push(Obj::Name(n, true))
}

fn with_exec(o: Obj, x: bool) -> Obj {
    match o {
        Obj::Name(n, _) => Obj::Name(n, x),
        Obj::Str(s, _) => Obj::Str(s, x),
        Obj::Arr(a, _) => Obj::Arr(a, x),
        Obj::File(f, _) => Obj::File(f, x),
        other => other,
    }
}

fn cvlit(vm: &mut Vm) -> R {
    let o = vm.pop()?;
    vm.push(with_exec(o, false))
}

fn cvx(vm: &mut Vm) -> R {
    let o = vm.pop()?;
    vm.push(with_exec(o, true))
}

fn xcheck(vm: &mut Vm) -> R {
    let x = matches!(
        vm.pop()?,
        Obj::Name(_, true) | Obj::Str(_, true) | Obj::Arr(_, true) | Obj::File(_, true) | Obj::Op(_)
    );
    vm.push_bool(x)
}

/// The number in a string (for `cvi` and `cvr`).
fn string_number(vm: &Vm, s: Slice) -> R<f64> {
    let b = vm.sbytes(s);
    let t = b
        .iter()
        .position(|&c| !super::scan::is_white(c))
        .map_or(&b[..0], |i| &b[i..]);
    let end = t.iter().position(|&c| super::scan::is_white(c)).unwrap_or(t.len());
    super::scan::number(&t[..end]).map_or(Err(PsError::TypeCheck), num)
}

fn cvi(vm: &mut Vm) -> R {
    let v = match vm.top(0)? {
        Obj::Str(s, _) => string_number(vm, s)?,
        o => num(o)?,
    };
    if v.abs() >= 2_147_483_648.0 {
        return Err(PsError::RangeCheck);
    }
    vm.pop()?;
    vm.push(Obj::Int(v.trunc() as i32))
}

fn cvr(vm: &mut Vm) -> R {
    let v = match vm.top(0)? {
        Obj::Str(s, _) => string_number(vm, s)?,
        o => num(o)?,
    };
    vm.pop()?;
    vm.push_real(v)
}

fn cvn(vm: &mut Vm) -> R {
    match vm.top(0)? {
        Obj::Str(s, x) => {
            let b = vm.sbytes(s).to_vec();
            let n = vm.intern(&b);
            vm.pop()?;
            vm.push(Obj::Name(n, x))
        }
        Obj::Name(..) => Ok(()),
        _ => Err(PsError::TypeCheck),
    }
}

/// The text of an object for `cvs`.
pub(super) fn text_of(vm: &Vm, o: Obj) -> Vec<u8> {
    match o {
        Obj::Int(i) => i.to_string().into_bytes(),
        Obj::Real(r) => real_text(r).into_bytes(),
        Obj::Bool(b) => b.to_string().into_bytes(),
        Obj::Str(s, _) => vm.sbytes(s).to_vec(),
        Obj::Name(n, _) => vm.name_bytes(n).to_vec(),
        Obj::Op(k) => vm.name_bytes(vm.op_names[k as usize]).to_vec(),
        _ => b"--nostringval--".to_vec(),
    }
}

fn real_text(r: f64) -> String {
    if r.fract() == 0.0 && r.abs() < 1e15 {
        format!("{r:.1}")
    } else {
        format!("{r}")
    }
}

/// Writes `text` into the start of string `s` and pushes that part.
fn push_into(vm: &mut Vm, s: Slice, x: bool, text: &[u8]) -> R {
    if text.len() > s.len as usize {
        return Err(PsError::RangeCheck);
    }
    let out = sub_slice(s, 0, text.len() as i64)?;
    vm.sbytes_mut(out).copy_from_slice(text);
    vm.push(Obj::Str(out, x))
}

fn cvs(vm: &mut Vm) -> R {
    let (o, s) = (vm.top(1)?, vm.top(0)?);
    let Obj::Str(s, x) = s else {
        return Err(PsError::TypeCheck);
    };
    let text = text_of(vm, o);
    if text.len() > s.len as usize {
        return Err(PsError::RangeCheck);
    }
    vm.drop_n(2);
    push_into(vm, s, x, &text)
}

fn cvrs(vm: &mut Vm) -> R {
    let (v, radix, s) = (vm.top(2)?, vm.int_at(1)?, vm.top(0)?);
    let Obj::Str(s, x) = s else {
        return Err(PsError::TypeCheck);
    };
    if !(2..=36).contains(&radix) {
        return Err(PsError::RangeCheck);
    }
    let text = if radix == 10 {
        match v {
            Obj::Real(r) => real_text(r).into_bytes(),
            o => int(o)?.to_string().into_bytes(),
        }
    } else {
        let mut u = num(v)?.trunc() as i64 as i32 as u32;
        let mut digits = Vec::new();
        loop {
            let d = (u % radix as u32) as u8;
            digits.push(if d < 10 { b'0' + d } else { b'A' + d - 10 });
            u /= radix as u32;
            if u == 0 {
                break;
            }
        }
        digits.reverse();
        digits
    };
    if text.len() > s.len as usize {
        return Err(PsError::RangeCheck);
    }
    vm.drop_n(3);
    push_into(vm, s, x, &text)
}

// ----- Control -----

fn exec(vm: &mut Vm) -> R {
    let o = vm.pop()?;
    vm.execute(o)
}

fn if_(vm: &mut Vm) -> R {
    let (b, p) = (vm.top(1)?, proc_of(vm.top(0)?)?);
    let Obj::Bool(b) = b else {
        return Err(PsError::TypeCheck);
    };
    vm.drop_n(2);
    if b { vm.execute(p) } else { Ok(()) }
}

fn ifelse(vm: &mut Vm) -> R {
    let (b, p, q) = (vm.top(2)?, proc_of(vm.top(1)?)?, proc_of(vm.top(0)?)?);
    let Obj::Bool(b) = b else {
        return Err(PsError::TypeCheck);
    };
    vm.drop_n(3);
    vm.execute(if b { p } else { q })
}

fn for_(vm: &mut Vm) -> R {
    let (a, i, l, p) = (vm.top(3)?, vm.top(2)?, vm.top(1)?, proc_of(vm.top(0)?)?);
    let frame = match (a, i, l) {
        (Obj::Int(a), Obj::Int(i), Obj::Int(l)) => Frame::ForInt {
            proc: p,
            cur: i64::from(a),
            inc: i64::from(i),
            lim: i64::from(l),
        },
        _ => Frame::ForReal {
            proc: p,
            cur: num(a)?,
            inc: num(i)?,
            lim: num(l)?,
        },
    };
    vm.drop_n(4);
    vm.push_frame(frame)
}

fn repeat(vm: &mut Vm) -> R {
    let (n, p) = (vm.int_at(1)?, proc_of(vm.top(0)?)?);
    if n < 0 {
        return Err(PsError::RangeCheck);
    }
    vm.drop_n(2);
    vm.push_frame(Frame::Repeat { proc: p, left: n })
}

fn loop_(vm: &mut Vm) -> R {
    let p = proc_of(vm.top(0)?)?;
    vm.pop()?;
    vm.push_frame(Frame::Loop { proc: p })
}

fn exit(vm: &mut Vm) -> R {
    vm.exit_loop()
}

fn stop(vm: &mut Vm) -> R {
    vm.stop()
}

fn stopped(vm: &mut Vm) -> R {
    let o = vm.pop()?;
    vm.push_frame(Frame::Stopped)?;
    vm.execute(o)
}

fn countexecstack(vm: &mut Vm) -> R {
    let n = vm.exec.len() as i64;
    vm.push_int(n)
}

fn execstack(vm: &mut Vm) -> R {
    let Obj::Arr(a, x) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    let n = (vm.exec.len() as i64).min(i64::from(a.len));
    vm.pop()?;
    vm.push(Obj::Arr(sub_slice(a, 0, n)?, x))
}

fn quit(_: &mut Vm) -> R {
    Err(PsError::Quit)
}

fn bind(vm: &mut Vm) -> R {
    let Obj::Arr(a, _) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    let mut todo = vec![a];
    let mut seen = std::collections::HashSet::new();
    while let Some(a) = todo.pop() {
        if !seen.insert((a.id, a.start, a.len)) {
            continue;
        }
        for i in 0..a.len as usize {
            match vm.arr(a)[i] {
                Obj::Name(n, true) => {
                    if let Some(Obj::Op(k)) = vm.lookup(n) {
                        vm.arr_mut(a)[i] = Obj::Op(k);
                    }
                }
                Obj::Arr(inner, true) => todo.push(inner),
                _ => {}
            }
        }
    }
    Ok(())
}

// ----- Memory and the system -----

fn save(vm: &mut Vm) -> R {
    let id = vm.saves.len() as u32;
    vm.saves.push(vm.gstack.len());
    super::graphics::gsave_by(vm, true);
    vm.push(Obj::Save(id))
}

fn restore(vm: &mut Vm) -> R {
    let Obj::Save(id) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    vm.pop()?;
    let Some(&len) = vm.saves.get(id as usize) else {
        return Err(PsError::InvalidAccess);
    };
    vm.saves.truncate(id as usize);
    if let Some(p) = vm.gstack.get(len) {
        vm.g = p.g.clone();
    }
    vm.gstack.truncate(len);
    Ok(())
}

fn vmstatus(vm: &mut Vm) -> R {
    let level = vm.saves.len() as i64;
    vm.push_int(level)?;
    vm.push_int(100_000)?;
    vm.push_int(10_000_000)
}

fn version(vm: &mut Vm) -> R {
    let s = vm.new_string(b"3010".to_vec())?;
    vm.push(Obj::Str(s, false))
}

fn languagelevel(vm: &mut Vm) -> R {
    vm.push(Obj::Int(2))
}

fn product(vm: &mut Vm) -> R {
    let s = vm.new_string(b"ShokujoDX".to_vec())?;
    vm.push(Obj::Str(s, false))
}

fn realtime(vm: &mut Vm) -> R {
    // From the steps, so each import gives the same result.
    let t = (vm.steps / 1000) as i64;
    vm.push_int(t)
}

fn internaldict(vm: &mut Vm) -> R {
    vm.pop()?;
    let d = match vm.dict_get_name(vm.systemdict, "ShokujoInternalDict") {
        Some(Obj::Dict(d)) => d,
        _ => {
            let d = vm.new_dict(16)?;
            let k = vm.name_obj("ShokujoInternalDict");
            let sys = vm.systemdict;
            vm.dict_put(sys, k, Obj::Dict(d))?;
            d
        }
    };
    vm.push(Obj::Dict(d))
}

fn cachestatus(vm: &mut Vm) -> R {
    for _ in 0..7 {
        vm.push(Obj::Int(0))?;
    }
    Ok(())
}

fn currentcacheparams(vm: &mut Vm) -> R {
    vm.push(Obj::Mark)?;
    vm.push(Obj::Int(0))?;
    vm.push(Obj::Int(0))
}

// ----- Files -----

/// Reads from file `f`: `take` gets the bytes from the position of the file
/// and gives its result and the number of bytes that it used.
pub(super) fn with_file<T>(vm: &mut Vm, f: u32, take: impl FnOnce(&[u8]) -> (T, usize)) -> R<T> {
    match f {
        0 => {
            let (frame, data, pos) = vm.text_rest().ok_or(PsError::IoError)?;
            let (t, used) = take(data.get(pos..).unwrap_or_default());
            vm.set_text_pos(frame, pos + used);
            Ok(t)
        }
        1 => Err(PsError::IoError),
        _ => {
            materialize(vm, f, 0)?;
            let file = vm.files.get_mut(f as usize).ok_or(PsError::IoError)?;
            let (t, used) = take(file.data.get(file.pos..).unwrap_or_default());
            file.pos += used;
            Ok(t)
        }
    }
}

fn file_of(o: Obj) -> R<u32> {
    match o {
        Obj::File(f, _) => Ok(f),
        _ => Err(PsError::TypeCheck),
    }
}

fn currentfile(vm: &mut Vm) -> R {
    vm.push(Obj::File(0, false))
}

fn read(vm: &mut Vm) -> R {
    let f = file_of(vm.top(0)?)?;
    vm.pop()?;
    let b = with_file(vm, f, |d| (d.first().copied(), usize::from(!d.is_empty())))?;
    match b {
        Some(b) => {
            vm.push(Obj::Int(i32::from(b)))?;
            vm.push_bool(true)
        }
        None => vm.push_bool(false),
    }
}

/// A reader of `readstring` and its kin: the bytes from the position of the
/// file and the size of the string give the bytes, the number used and the
/// result value.
type ReadFn = fn(&[u8], usize) -> (Vec<u8>, usize, bool);

/// Reads into a string with `fill` (which gives the bytes, the number of
/// bytes used and the result value), and pushes the part that it filled
/// and the value.
fn read_into(vm: &mut Vm, fill: ReadFn) -> R {
    let (f, s) = (file_of(vm.top(1)?)?, vm.top(0)?);
    let Obj::Str(s, x) = s else {
        return Err(PsError::TypeCheck);
    };
    vm.drop_n(2);
    let want = s.len as usize;
    let (got, ok) = with_file(vm, f, |d| {
        let (got, used, ok) = fill(d, want);
        ((got, ok), used)
    })?;
    push_into(vm, s, x, &got)?;
    vm.push_bool(ok)
}

fn readhexstring(vm: &mut Vm) -> R {
    read_into(vm, |d, want| {
        let mut out = Vec::with_capacity(want);
        let mut high: Option<u8> = None;
        let mut used = 0;
        for &c in d {
            if out.len() == want {
                break;
            }
            used += 1;
            let Some(v) = (c as char).to_digit(16) else {
                continue;
            };
            match high.take() {
                Some(h) => out.push(h << 4 | v as u8),
                None => high = Some(v as u8),
            }
        }
        let full = out.len() == want;
        (out, used, full)
    })
}

fn readstring(vm: &mut Vm) -> R {
    read_into(vm, |d, want| {
        let n = want.min(d.len());
        (d[..n].to_vec(), n, n == want)
    })
}

fn readline(vm: &mut Vm) -> R {
    read_into(vm, |d, want| {
        let end = d.iter().position(|&c| c == b'\n' || c == b'\r').unwrap_or(d.len());
        let n = end.min(want);
        let mut used = n;
        // `true` if the line ends with a new line.
        let ended = n == end && end < d.len();
        if ended {
            used += if d[end] == b'\r' && d.get(end + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        }
        (d[..n].to_vec(), used, ended)
    })
}

fn bytesavailable(vm: &mut Vm) -> R {
    vm.pop()?;
    vm.push(Obj::Int(-1))
}

fn closefile(vm: &mut Vm) -> R {
    let f = file_of(vm.top(0)?)?;
    vm.pop()?;
    with_file(vm, f, |d| ((), d.len())).or(Ok(()))
}

fn status(vm: &mut Vm) -> R {
    match vm.pop()? {
        Obj::File(..) => vm.push_bool(true),
        _ => vm.push_bool(false),
    }
}

fn file(vm: &mut Vm) -> R {
    let Obj::Str(name, _) = vm.top(1)? else {
        return Err(PsError::TypeCheck);
    };
    let name = vm.sbytes(name).to_vec();
    vm.drop_n(2);
    if name.starts_with(b"%stdout") || name.starts_with(b"%stderr") {
        vm.push(Obj::File(1, false))
    } else if name.starts_with(b"%stdin") {
        vm.push(Obj::File(0, false))
    } else {
        Err(PsError::IoError)
    }
}

/// The most filters in a chain.
const MAX_FILTER_CHAIN: usize = 32;

/// Decodes the source of filter `f` if it did not read it yet.
pub(super) fn materialize(vm: &mut Vm, f: u32, depth: usize) -> R {
    let Some(p) = vm.files.get_mut(f as usize).and_then(|file| file.pending.take()) else {
        return Ok(());
    };
    if depth > MAX_FILTER_CHAIN {
        return Err(PsError::LimitCheck);
    }
    if let Obj::File(g, _) = p.source
        && g > 1
    {
        materialize(vm, g, depth + 1)?;
    }
    let data = decode(vm, &p)?;
    vm.charge(data.len())?;
    vm.files[f as usize].data = data;
    Ok(())
}

/// The bytes of a filter source: what is left of a file (the encoded part
/// of the main file, with `end` to find where it ends), or a string.
fn source_bytes(vm: &mut Vm, src: Obj, end: fn(&[u8]) -> (Vec<u8>, usize)) -> R<Vec<u8>> {
    match src {
        Obj::File(f, _) => with_file(vm, f, end),
        Obj::Str(s, _) => Ok(end(vm.sbytes(s)).0),
        _ => Err(PsError::Unsupported),
    }
}

fn filter(vm: &mut Vm) -> R {
    let Obj::Name(kind, _) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    let kind = vm.name_text(kind);
    let mut n = 1;
    let (mut count, mut eod) = (0, Vec::new());
    if kind == "SubFileDecode" {
        n += 2;
        count = vm.int_at(2)?.max(0) as usize;
        let Obj::Str(s, _) = vm.top(1)? else {
            return Err(PsError::TypeCheck);
        };
        eod = vm.sbytes(s).to_vec();
    } else if matches!(vm.top(1)?, Obj::Dict(_)) && vm.ops.len() > 2 {
        n += 1;
    }
    vm.need(n + 1)?;
    let source = vm.top(n)?;
    vm.drop_n(n + 1);
    if kind.ends_with("Encode") {
        // Writing: nothing is kept.
        return vm.push(Obj::File(1, false));
    }
    if !matches!(source, Obj::File(..) | Obj::Str(..)) {
        return Err(PsError::Unsupported);
    }
    let id = vm.files.len() as u32;
    vm.charge(64)?;
    vm.files.push(super::vm::FileData {
        data: Vec::new(),
        pos: 0,
        pending: Some(super::vm::Pending {
            kind,
            source,
            count,
            eod,
        }),
    });
    vm.push(Obj::File(id, false))
}

/// The decoded bytes of a filter.
fn decode(vm: &mut Vm, p: &super::vm::Pending) -> R<Vec<u8>> {
    let src = p.source;
    Ok(match p.kind.as_str() {
        "ASCII85Decode" => source_bytes(vm, src, |d| filters::ascii85(d, 0))?,
        "ASCIIHexDecode" => source_bytes(vm, src, |d| filters::ascii_hex(d, 0))?,
        "RunLengthDecode" => source_bytes(vm, src, |d| filters::run_length(d, 0))?,
        "FlateDecode" => source_bytes(vm, src, |d| filters::flate(d, 0))?,
        "SubFileDecode" => {
            let (count, eod) = (p.count, p.eod.clone());
            let take = move |d: &[u8]| -> (Vec<u8>, usize) {
                if eod.is_empty() {
                    let n = count.min(d.len());
                    return (d[..n].to_vec(), n);
                }
                let mut from = 0;
                let mut seen = 0;
                while let Some(i) = d[from..].windows(eod.len()).position(|w| w == eod.as_slice()) {
                    if seen == count {
                        return (d[..from + i].to_vec(), from + i + eod.len());
                    }
                    seen += 1;
                    from += i + eod.len();
                }
                (d.to_vec(), d.len())
            };
            match src {
                Obj::File(f, _) => with_file(vm, f, take)?,
                Obj::Str(s, _) => take(vm.sbytes(s)).0,
                _ => return Err(PsError::Unsupported),
            }
        }
        "ReusableStreamDecode" | "NullDecode" => source_bytes(vm, src, |d| (d.to_vec(), d.len()))?,
        "DCTDecode" => {
            // Only the end of the picture is needed: the end marker.
            source_bytes(vm, src, |d| {
                let end = d.windows(2).position(|w| w == [0xff, 0xd9]).map_or(d.len(), |i| i + 2);
                (Vec::new(), end)
            })?
        }
        _ => match src {
            // The data of another filter: the reader does not decode it.
            Obj::File(f, _) if f > 1 => with_file(vm, f, |d| (Vec::new(), d.len()))?,
            Obj::Str(_, _) => Vec::new(),
            _ => return Err(PsError::Unsupported),
        },
    })
}

/// `eexec`: the encrypted part of a Type 1 font. The reader does not draw
/// text, so it does not decrypt the part: it defines the font that is on
/// the stack and goes on after the zeros at the end of the part (the
/// encrypted part ends with `mark currentfile closefile`).
fn eexec(vm: &mut Vm) -> R {
    let src = vm.pop()?;
    if let Obj::File(f, _) = src {
        with_file(vm, f, |d| {
            let zeros = [b'0'; 64];
            let mut end = d.windows(64).position(|w| w == zeros).unwrap_or(d.len());
            while end < d.len() && (d[end] == b'0' || super::scan::is_white(d[end])) {
                end += 1;
            }
            ((), end)
        })?;
    }
    if let Ok(Obj::Dict(font)) = vm.top(0) {
        vm.pop()?;
        let name = vm
            .dict_get_name(font, "FontName")
            .unwrap_or_else(|| vm.name_obj("Font"));
        super::graphics::define_font(vm, name, font)?;
    }
    vm.push(Obj::Mark)
}

// ----- Resources -----

/// The dictionary of a resource category (made when it is needed).
fn category(vm: &mut Vm, cat: Obj) -> R<u32> {
    let r = vm.resources;
    match vm.dict_get(r, cat)? {
        Some(Obj::Dict(d)) => Ok(d),
        _ => {
            let d = vm.new_dict(16)?;
            vm.dict_put(r, cat, Obj::Dict(d))?;
            Ok(d)
        }
    }
}

fn is_name(vm: &Vm, o: Obj, s: &str) -> bool {
    matches!(o, Obj::Name(n, _) if vm.name_bytes(n) == s.as_bytes())
}

fn findresource(vm: &mut Vm) -> R {
    let (key, cat) = (vm.top(1)?, vm.top(0)?);
    if is_name(vm, cat, "Font") {
        vm.pop()?;
        return super::graphics::findfont(vm);
    }
    let d = category(vm, cat)?;
    let found = match vm.dict_get(d, key)? {
        Some(v) => Some(v),
        None if is_name(vm, cat, "Encoding") => match key {
            Obj::Name(n, _) => vm.lookup(n),
            _ => None,
        },
        None if is_name(vm, cat, "Category") => Some(Obj::Dict(category(vm, key)?)),
        None => None,
    };
    let Some(v) = found else {
        return Err(PsError::UndefinedResource);
    };
    vm.drop_n(2);
    vm.push(v)
}

fn defineresource(vm: &mut Vm) -> R {
    let (key, inst, cat) = (vm.top(2)?, vm.top(1)?, vm.top(0)?);
    if is_name(vm, cat, "Font")
        && let Obj::Dict(font) = inst
    {
        vm.drop_n(3);
        super::graphics::define_font(vm, key, font)?;
        return vm.push(inst);
    }
    let d = category(vm, cat)?;
    vm.dict_put(d, key, inst)?;
    vm.drop_n(3);
    vm.push(inst)
}

fn undefineresource(vm: &mut Vm) -> R {
    let (key, cat) = (vm.top(1)?, vm.top(0)?);
    let d = category(vm, cat)?;
    vm.dict_undef(d, key)?;
    vm.drop_n(2);
    Ok(())
}

fn resourcestatus(vm: &mut Vm) -> R {
    let (key, cat) = (vm.top(1)?, vm.top(0)?);
    let found = if is_name(vm, cat, "Font") {
        let fd = vm.fontdir;
        vm.dict_get(fd, key)?.is_some()
    } else {
        let d = category(vm, cat)?;
        vm.dict_get(d, key)?.is_some()
    };
    vm.drop_n(2);
    if found {
        vm.push(Obj::Int(1))?;
        vm.push(Obj::Int(0))?;
    }
    vm.push_bool(found)
}

fn resourceforall(vm: &mut Vm) -> R {
    vm.need(4)?;
    vm.drop_n(4);
    Ok(())
}

fn findencoding(vm: &mut Vm) -> R {
    let k = vm.top(0)?;
    let Obj::Name(n, _) = k else {
        return Err(PsError::TypeCheck);
    };
    let v = vm.lookup(n).ok_or(PsError::UndefinedResource)?;
    vm.pop()?;
    vm.push(v)
}
