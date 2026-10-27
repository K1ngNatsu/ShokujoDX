//! Color reduction ("prepare colors"): a picture with a small number of
//! colors, ready to trace or to digitize by hand.
//!
//! 1. The colors are counted in a histogram (5 bits for each channel).
//! 2. K-means in Lab color space on the histogram finds the colors. It
//!    starts with the most common color, then each time the color farthest
//!    from the colors so far (weighted by its count), so the result is the
//!    same each time.
//!    With a thread chart ([`reduce_with`], P12 Milestone 3), each color
//!    found goes to the nearest thread of the chart; colors that go to one
//!    thread are one color, and each pixel takes the nearest of the threads.
//! 3. Specks (connected areas of one color smaller than a size) take the
//!    color that is most common round them.
//! 4. Clean edges: a pixel with six or more neighbors of one other color
//!    takes that color.
//!
//! Transparent pixels (alpha < 128) have no color ([`NONE`]).

use std::collections::BTreeMap;

use emb_model::{Lab, Rgb, ThreadChart};
use image::{Rgba, RgbaImage};

/// The label of a transparent pixel.
pub const NONE: u16 = u16::MAX;
/// K-means stops after this many rounds.
const MAX_ROUNDS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReduceSpec {
    /// The number of colors, 2 to 32.
    pub colors: usize,
    /// Areas of one color with fewer pixels than this take the color round
    /// them (clear counts as a color, but a clear area at the edge stays).
    /// 0: keep all.
    pub min_speck_px: usize,
    pub clean_edges: bool,
}

impl Default for ReduceSpec {
    fn default() -> Self {
        ReduceSpec {
            colors: 4,
            min_speck_px: 16,
            clean_edges: true,
        }
    }
}

/// A picture with a few colors: a color index for each pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced {
    pub width: u32,
    pub height: u32,
    /// The colors, the most common first.
    pub palette: Vec<Rgb>,
    /// The index in `palette` of each pixel, row by row; [`NONE`] where the
    /// picture is transparent.
    pub labels: Vec<u16>,
}

impl Reduced {
    /// The picture in its palette colors.
    pub fn to_image(&self) -> RgbaImage {
        RgbaImage::from_fn(self.width, self.height, |x, y| {
            match self.labels[(y * self.width + x) as usize] {
                NONE => Rgba([0, 0, 0, 0]),
                l => {
                    let c = self.palette[l as usize];
                    Rgba([c.0, c.1, c.2, 255])
                }
            }
        })
    }

    /// The number of pixels of each color.
    pub fn counts(&self) -> Vec<usize> {
        let mut n = vec![0; self.palette.len()];
        for &l in &self.labels {
            if l != NONE {
                n[l as usize] += 1;
            }
        }
        n
    }
}

/// The colors of a picture as they are, if it has `max_colors` or fewer
/// (a picture that is already reduced, or flat artwork). The palette has
/// the most common color first. `None` if it has more colors.
pub fn exact(img: &RgbaImage, max_colors: usize) -> Option<Reduced> {
    let mut count: BTreeMap<[u8; 3], usize> = BTreeMap::new();
    for p in img.pixels().filter(|p| p[3] >= 128) {
        *count.entry([p[0], p[1], p[2]]).or_default() += 1;
        if count.len() > max_colors {
            return None;
        }
    }
    let mut colors: Vec<([u8; 3], usize)> = count.into_iter().collect();
    // Most common first; equal counts in color order, so the result is the same each time.
    colors.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let index: BTreeMap<[u8; 3], u16> = colors.iter().enumerate().map(|(i, c)| (c.0, i as u16)).collect();
    let labels = img
        .pixels()
        .map(|p| if p[3] >= 128 { index[&[p[0], p[1], p[2]]] } else { NONE })
        .collect();
    Some(Reduced {
        width: img.width(),
        height: img.height(),
        palette: colors.iter().map(|(c, _)| Rgb(c[0], c[1], c[2])).collect(),
        labels,
    })
}

/// The nearest thread of `chart` to each color.
pub fn match_threads(palette: &[Rgb], chart: &ThreadChart) -> Vec<Option<usize>> {
    palette.iter().map(|c| chart.nearest(*c)).collect()
}

/// One histogram bin: the sum of its colors and its count.
#[derive(Debug, Clone, Copy, Default)]
struct Bin {
    sum: [f64; 3],
    count: f64,
}

impl Bin {
    fn rgb(&self) -> Rgb {
        let c = |i: usize| (self.sum[i] / self.count).round().clamp(0.0, 255.0) as u8;
        Rgb(c(0), c(1), c(2))
    }
}

fn key(p: &Rgba<u8>) -> u16 {
    (u16::from(p[0] >> 3) << 10) | (u16::from(p[1] >> 3) << 5) | u16::from(p[2] >> 3)
}

/// Reduces the colors of `img` (see the module notes).
pub fn reduce(img: &RgbaImage, spec: &ReduceSpec) -> Reduced {
    reduce_with(img, spec, None)
}

/// As [`reduce`]; with `chart` (the colors of a thread chart), the colors
/// are threads of the chart.
pub fn reduce_with(img: &RgbaImage, spec: &ReduceSpec, chart: Option<&[Rgb]>) -> Reduced {
    let (w, h) = img.dimensions();
    let k = spec.colors.clamp(2, 32);
    // 1. Histogram.
    let mut hist: BTreeMap<u16, Bin> = BTreeMap::new();
    for p in img.pixels().filter(|p| p[3] >= 128) {
        let b = hist.entry(key(p)).or_default();
        for c in 0..3 {
            b.sum[c] += f64::from(p[c]);
        }
        b.count += 1.0;
    }
    let keys: Vec<u16> = hist.keys().copied().collect();
    let bins: Vec<Bin> = hist.values().copied().collect();
    let labs: Vec<Lab> = bins.iter().map(|b| Lab::of(b.rgb())).collect();
    // 2. K-means on the bins.
    let mut centers: Vec<Lab> = Vec::new();
    if let Some(first) = (0..bins.len()).max_by(|&a, &b| bins[a].count.total_cmp(&bins[b].count).then(b.cmp(&a))) {
        centers.push(labs[first]);
    }
    while centers.len() < k.min(bins.len()) {
        let score = |i: usize| {
            let d = centers
                .iter()
                .map(|c| c.distance(labs[i]))
                .fold(f64::INFINITY, f64::min);
            d * bins[i].count.sqrt()
        };
        let Some(next) = (0..bins.len()).max_by(|&a, &b| score(a).total_cmp(&score(b)).then(b.cmp(&a))) else {
            break;
        };
        if score(next) <= 0.0 {
            break;
        }
        centers.push(labs[next]);
    }
    let nearest = |centers: &[Lab], l: Lab| {
        (0..centers.len())
            .min_by(|&a, &b| centers[a].distance(l).total_cmp(&centers[b].distance(l)))
            .unwrap_or(0)
    };
    let mut assign: Vec<usize> = labs.iter().map(|l| nearest(&centers, *l)).collect();
    let mut means: Vec<Bin> = Vec::new();
    for _ in 0..MAX_ROUNDS {
        means = vec![Bin::default(); centers.len()];
        for (i, b) in bins.iter().enumerate() {
            let m = &mut means[assign[i]];
            for c in 0..3 {
                m.sum[c] += b.sum[c];
            }
            m.count += b.count;
        }
        for (c, m) in centers.iter_mut().zip(&means) {
            if m.count > 0.0 {
                *c = Lab::of(m.rgb());
            }
        }
        let next: Vec<usize> = labs.iter().map(|l| nearest(&centers, *l)).collect();
        if next == assign {
            break;
        }
        assign = next;
    }
    // The threads of the chart in place of the colors found.
    let mut threads: Option<Vec<Rgb>> = None;
    if let Some(chart) = chart.filter(|c| !c.is_empty()) {
        let chart_labs: Vec<Lab> = chart.iter().map(|c| Lab::of(*c)).collect();
        let mut picks: Vec<usize> = centers
            .iter()
            .zip(&means)
            .filter(|(_, m)| m.count > 0.0)
            .map(|(c, _)| nearest(&chart_labs, *c))
            .collect();
        picks.sort_unstable();
        picks.dedup();
        centers = picks.iter().map(|&i| chart_labs[i]).collect();
        assign = labs.iter().map(|l| nearest(&centers, *l)).collect();
        means = vec![Bin::default(); centers.len()];
        for (i, b) in bins.iter().enumerate() {
            means[assign[i]].count += b.count;
        }
        threads = Some(picks.iter().map(|&i| chart[i]).collect());
    }
    // The palette, the most common color first, with no empty colors.
    let mut order: Vec<usize> = (0..means.len()).filter(|&i| means[i].count > 0.0).collect();
    order.sort_by(|&a, &b| means[b].count.total_cmp(&means[a].count).then(a.cmp(&b)));
    let palette: Vec<Rgb> = order
        .iter()
        .map(|&i| threads.as_ref().map_or_else(|| means[i].rgb(), |t| t[i]))
        .collect();
    let mut remap = vec![NONE; means.len()];
    for (new, &old) in order.iter().enumerate() {
        remap[old] = new as u16;
    }
    let bin_label: BTreeMap<u16, u16> = keys.iter().zip(&assign).map(|(k, a)| (*k, remap[*a])).collect();
    let mut labels: Vec<u16> = img
        .pixels()
        .map(|p| if p[3] >= 128 { bin_label[&key(p)] } else { NONE })
        .collect();
    // 3 and 4.
    if spec.min_speck_px > 1 {
        for _ in 0..4 {
            if !merge_specks(&mut labels, w, h, spec.min_speck_px) {
                break;
            }
        }
    }
    if spec.clean_edges {
        labels = clean_edges(&labels, w, h);
    }
    Reduced {
        width: w,
        height: h,
        palette,
        labels,
    }
}

/// Gives each connected area smaller than `min` pixels the color most
/// common round it. Returns `true` if a pixel changed.
/// Clear counts as a color here (audit of P11 and P12): a small clear
/// area inside the picture (a hole of one or a few pixels, which made fills
/// with hundreds of holes) takes the color round it, and a small area of
/// color in the clear goes; a clear area that touches the edge of the
/// picture stays (it is the background).
fn merge_specks(labels: &mut [u16], w: u32, h: u32, min: usize) -> bool {
    let (w, h) = (w as usize, h as usize);
    let mut seen = vec![false; labels.len()];
    let mut changed = false;
    let mut stack = Vec::new();
    for start in 0..labels.len() {
        if seen[start] {
            continue;
        }
        let l = labels[start];
        let mut area = Vec::new();
        let mut round: BTreeMap<u16, usize> = BTreeMap::new();
        let mut at_edge = false;
        stack.push(start);
        seen[start] = true;
        while let Some(i) = stack.pop() {
            area.push(i);
            let (x, y) = (i % w, i / w);
            at_edge |= x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let mut near = |j: usize| {
                if labels[j] == l {
                    if !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                } else {
                    *round.entry(labels[j]).or_default() += 1;
                }
            };
            if x > 0 {
                near(i - 1);
            }
            if x + 1 < w {
                near(i + 1);
            }
            if y > 0 {
                near(i - w);
            }
            if y + 1 < h {
                near(i + w);
            }
        }
        if area.len() < min
            && !(l == NONE && at_edge)
            && let Some((&to, _)) = round.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        {
            for i in area {
                labels[i] = to;
            }
            changed = true;
        }
    }
    changed
}

/// A pixel with six or more of its eight neighbors in one other color
/// takes that color.
fn clean_edges(labels: &[u16], w: u32, h: u32) -> Vec<u16> {
    let (w, h) = (w as i64, h as i64);
    let mut out = labels.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if labels[i] == NONE {
                continue;
            }
            let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let l = labels[(ny * w + nx) as usize];
                    if l != NONE && l != labels[i] {
                        *counts.entry(l).or_default() += 1;
                    }
                }
            }
            if let Some((&l, _)) = counts.iter().find(|(_, n)| **n >= 6) {
                out[i] = l;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 40 × 20: the left half red, the right half blue, with small changes
    /// of color, one 2 × 2 white speck and a transparent corner pixel.
    fn picture() -> RgbaImage {
        RgbaImage::from_fn(40, 20, |x, y| {
            if (x, y) == (0, 0) {
                return Rgba([0, 0, 0, 0]);
            }
            if (5..7).contains(&x) && (5..7).contains(&y) {
                return Rgba([255, 255, 255, 255]);
            }
            let n = ((x * 7 + y * 13) % 9) as u8;
            if x < 20 {
                Rgba([200 + n, 20 + n, 30, 255])
            } else {
                Rgba([20, 40 + n, 200 - n, 255])
            }
        })
    }

    #[test]
    fn two_colors_with_no_speck() {
        let r = reduce(
            &picture(),
            &ReduceSpec {
                colors: 2,
                min_speck_px: 8,
                clean_edges: true,
            },
        );
        assert_eq!(r.palette.len(), 2);
        let red = r.labels[(10 * 40 + 10) as usize];
        let blue = r.labels[(10 * 40 + 30) as usize];
        assert_ne!(red, blue);
        assert!(
            r.palette[red as usize].0 > 190 && r.palette[blue as usize].2 > 180,
            "{:?}",
            r.palette
        );
        // The white speck took the red color round it.
        assert_eq!(r.labels[(5 * 40 + 5) as usize], red);
        assert_eq!(r.labels[0], NONE);
        let img = r.to_image();
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(r.counts().iter().sum::<usize>(), 40 * 20 - 1);
    }

    #[test]
    fn more_colors_keep_the_speck_and_the_result_is_the_same_each_time() {
        let spec = ReduceSpec {
            colors: 3,
            min_speck_px: 0,
            clean_edges: false,
        };
        let a = reduce(&picture(), &spec);
        assert_eq!(a.palette.len(), 3);
        assert!(a.palette.contains(&Rgb(255, 255, 255)), "{:?}", a.palette);
        assert_eq!(a, reduce(&picture(), &spec));
        // A chart thread for each color.
        let chart = &ThreadChart::builtin()[0];
        assert!(match_threads(&a.palette, chart).iter().all(Option::is_some));
    }

    #[test]
    fn fewer_colors_in_the_picture_than_asked() {
        let img = RgbaImage::from_pixel(5, 5, Rgba([10, 200, 10, 255]));
        let r = reduce(&img, &ReduceSpec::default());
        assert_eq!(r.palette.len(), 1);
    }

    #[test]
    fn exact_keeps_the_colors_of_a_reduced_picture() {
        let r = reduce(
            &picture(),
            &ReduceSpec {
                colors: 2,
                min_speck_px: 8,
                clean_edges: true,
            },
        );
        // The colors of the reduced picture, as they are.
        let e = exact(&r.to_image(), 32).unwrap();
        assert_eq!(e, r);
        // A gradient of 64 colors has too many.
        let gradient = RgbaImage::from_fn(64, 2, |x, _| Rgba([4 * x as u8, 0, 0, 255]));
        assert!(exact(&gradient, 32).is_none());
        assert_eq!(exact(&gradient, 64).unwrap().palette.len(), 64);
    }

    #[test]
    fn colors_from_a_thread_chart() {
        // Two reds that are near and a blue; the chart has one red.
        let img = RgbaImage::from_fn(30, 10, |x, _| match x / 10 {
            0 => Rgba([200, 30, 30, 255]),
            1 => Rgba([215, 45, 40, 255]),
            _ => Rgba([30, 40, 190, 255]),
        });
        let chart = [Rgb(205, 35, 35), Rgb(20, 40, 200), Rgb(250, 250, 250), Rgb(0, 0, 0)];
        let spec = ReduceSpec {
            colors: 3,
            min_speck_px: 0,
            clean_edges: false,
        };
        assert_eq!(reduce(&img, &spec).palette.len(), 3);
        let r = reduce_with(&img, &spec, Some(&chart));
        assert_eq!(r.palette, vec![Rgb(205, 35, 35), Rgb(20, 40, 200)]);
        assert_eq!(r.labels[0], r.labels[15]);
        assert_ne!(r.labels[0], r.labels[25]);
    }

    #[test]
    fn clear_specks_fill_and_color_specks_in_the_clear_go() {
        // Red with a clear pixel in it; a red pixel in the clear on the
        // right; the clear right part touches the edge and stays.
        let img = RgbaImage::from_fn(20, 10, |x, y| match (x, y) {
            (5, 5) => Rgba([0, 0, 0, 0]),
            (15, 5) => Rgba([200, 30, 30, 255]),
            (x, _) if x >= 10 => Rgba([0, 0, 0, 0]),
            _ => Rgba([200, 30, 30, 255]),
        });
        let r = reduce(
            &img,
            &ReduceSpec {
                colors: 2,
                min_speck_px: 4,
                clean_edges: false,
            },
        );
        assert_ne!(r.labels[5 * 20 + 5], NONE, "the hole fills");
        assert_eq!(r.labels[5 * 20 + 15], NONE, "the speck goes");
        assert_eq!(r.labels[19], NONE, "the clear background stays");
    }
}
