// TODO remove these
#![allow(dead_code)]
#![allow(unused_variables)]

// use std::cmp::min;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;

const BLACK: Pixel = Pixel { r: 0, g: 0, b: 0, a: 255 };
const BLACKT: Pixel = Pixel { r: 0, g: 0, b: 0, a: 128 };
const NONE: Pixel = Pixel { r: 0, g: 0, b: 0, a: 0 };
const REDT: Pixel = Pixel { r: 255, g: 0, b: 0, a: 128 };

pub fn length(a: f32, b: f32) -> f32 {
  (a*a + b*b).sqrt()
}

#[derive(Clone, Copy, Debug)]
pub struct Pixel {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 255, g: 0, b: 0, a: 128 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 255, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 255, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 128 } Pixel { r: 255, g: 0, b: 0, a: 255 }
// a over b
pub fn over(a: &Pixel, b: &Pixel) -> Pixel {
  let ao = (a.a as f32 + (b.a as f32 * (255.0 - a.a as f32))).clamp(0.0, 255.0);
  let over = Pixel {
    a: ao as u8,
    r: (((a.r as f32 * a.a as f32) + (b.r as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
    g: (((a.g as f32 * a.a as f32) + (b.g as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
    b: (((a.b as f32 * a.a as f32) + (b.b as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
  };
  // println!("OVER {:?} {:?} {:?}", a, b, over);
  over
}

#[derive(Clone, Copy)]
pub struct Pt<T> {
  pub x: T,
  pub y: T,
}

#[derive(Clone, Copy)]
pub struct Rect<T> {
  pub ll: Pt<T>,
  pub ur: Pt<T>,
}

#[allow(unused_variables, dead_code)]
pub struct FB {
  pub w: usize,
  pub h: usize,
  pixels: Vec<u8>,
}

impl FB {
  pub fn new(w: usize, h: usize) -> FB {
    let fb = FB {
      w: w,
      h: h,
      pixels: vec![0; w*h*4],
    };
    return fb;
  }

  pub fn set(&mut self, x: usize, y: usize, pix: &Pixel) {
    let off = ((y*self.w) + x) * 4;
    // TODO unnecessary copy
    let current = Pixel { r: self.pixels[off+0], b: self.pixels[off+1], g: self.pixels[off+2], a: self.pixels[off+3] };
    let over = over(&pix, &current);
    self.pixels[off+0] = over.r;
    self.pixels[off+1] = over.g;
    self.pixels[off+2] = over.b;
    self.pixels[off+3] = over.a;
  }

  pub fn write(&self, path: String) {
    // let path = Path::new(r"image.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, self.w.try_into().unwrap(), self.h.try_into().unwrap());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_trns(vec!(0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8));
    encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
    encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
    let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
        (0.31270, 0.32900),
        (0.64000, 0.33000),
        (0.30000, 0.60000),
        (0.15000, 0.06000)
    );
    encoder.set_source_chromaticities(source_chromaticities);
    let mut writer = encoder.write_header().unwrap();

    // let arr = Box::new([u8; self.w * self.h * 4]);
    let sl: &[u8] = &self.pixels;
  // let a = vec![1, 2, 3, 4, 5];
  //   let b: &[i32] = &a;
    writer.write_image_data(sl).unwrap(); // Save

    // let data = self.pixels.try_into()
    //     .unwrap_or_else(|v: Vec<u8>| panic!("oops"));

    // writer.write_image_data(&data).unwrap(); // Save
  }

  // pub fn get_raw<const N: usize>(&self) -> [u8; N] {
  //   // return self.pixels.try_into().unwrap();
  //   self.pixels.try_into()
  //       .unwrap_or_else(|v: Vec<u8>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
  // }
}

fn render(shape: &impl Shape, colorer: fn(f32) -> Pixel, domain: Rect<f32>, fb: &mut FB) {
  let ox = domain.ll.x;
  let oy = domain.ll.y;
  let dx = (domain.ur.x - domain.ll.x) / (fb.w as f32);
  let dy = (domain.ur.y - domain.ll.y) / (fb.h as f32);
  for x in 0..fb.w {
    for y in 0..fb.h {
      let fx = ox + ((x as f32) * dx);
      let fy = oy + ((y as f32) * dy);
      fb.set(x, y, &colorer(shape.dist(fx, fy)));
    }
  }
}

pub trait Shape {
  fn dist(&self, x: f32, y: f32) -> f32;
}

// TODO center, radius?
#[derive(Clone, Copy, Debug)]
pub struct Circle {
}

impl Shape for Circle {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let d = length(x, y) - 1.0;
    // println!("dist {} {} {}|", x, y, d);
    d
  }
}

pub struct Translate {
  shape: Box<dyn Shape>,
  tx: f32,
  ty: f32,
}

impl Translate {
  pub fn new(shape: Box<dyn Shape>, tx: f32, ty: f32) -> Translate {
    Translate { shape: shape, tx: tx, ty: ty }
  }
}

impl Shape for Translate {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x - self.tx, y - self.ty)
  }
}

pub struct Scale {
  shape: Box<dyn Shape>,
  sx: f32,
  sy: f32,
}

impl Scale {
  pub fn new(shape: Box<dyn Shape>, sx: f32, sy: f32) -> Scale {
    Scale { shape: shape, sx: sx, sy: sy }
  }
}

impl Shape for Scale {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x / self.sx, y / self.sy)
  }
}

pub struct Union {
  shape0: Box<dyn Shape>,
  shape1: Box<dyn Shape>,
}

impl Union {
  pub fn new(shape0: Box<dyn Shape>, shape1: Box<dyn Shape>) -> Union {
    Union { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Union {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).min(self.shape1.dist(x, y))
  }
}

pub struct Intersection {
  shape0: Box<dyn Shape>,
  shape1: Box<dyn Shape>,
}

impl Intersection {
  pub fn new(shape0: Box<dyn Shape>, shape1: Box<dyn Shape>) -> Intersection {
    Intersection { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Intersection {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(self.shape1.dist(x, y))
  }
}

pub struct Difference {
  shape0: Box<dyn Shape>,
  shape1: Box<dyn Shape>,
}

impl Difference {
  pub fn new(shape0: Box<dyn Shape>, shape1: Box<dyn Shape>) -> Difference {
    Difference { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Difference {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(-self.shape1.dist(x, y))
  }
}

pub struct SmoothUnion {
  shape0: Box<dyn Shape>,
  shape1: Box<dyn Shape>,
}

impl SmoothUnion {
  pub fn new(shape0: Box<dyn Shape>, shape1: Box<dyn Shape>) -> SmoothUnion {
    SmoothUnion { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for SmoothUnion {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let d0 = self.shape0.dist(x, y);
    let d1 = self.shape1.dist(x, y);
    length((d0-0.1).min(0.0), (d1-0.1).min(0.0))
  }
}

// float round_merge(float shape1, float shape2, float radius){
//     float2 intersectionSpace = float2(shape1, shape2);
//     intersectionSpace = min(intersectionSpace, 0);
//     return length(intersectionSpace);
// }

pub struct Hmm {
  shape0: Box<dyn Shape>,
  shape1: Box<dyn Shape>,
}

impl Hmm {
  pub fn new(shape0: Box<dyn Shape>, shape1: Box<dyn Shape>) -> Hmm {
    Hmm { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Hmm {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y) + self.shape1.dist(x, y)
  }
}

pub struct Grid {
  w: f32,
  h: f32,
  shape: Box<dyn Shape>,
}

impl Grid {
  pub fn new(w: f32, h: f32, shape: Box<dyn Shape>) -> Grid {
    Grid { w: w, h: h, shape: shape }
  }
}

fn grid_fmod(a: f32, b: f32) -> f32 {
  let aob = a / b;
  (aob - aob.floor()) * b
}

impl Shape for Grid {
  fn dist(&self, x: f32, y: f32) -> f32 {
    // let ix = (x / self.w).floor();
    // let iy = (y / self.h).floor();
    // println!("FMOD {} {} {}", 0.5 % 0.3, 12.0 % 5.0, -0.5 % 0.3);
    // println!("FLOOR {} {}", (1.6_f32).floor(), (-1.6_f32).floor());
    // println!("FMOD {} {} {} {} {} {}", grid_fmod(0.3, 0.3), grid_fmod(0.45, 0.3), grid_fmod(0.6, 0.3), grid_fmod(0.5, 0.3), grid_fmod(12.0, 5.0), grid_fmod(-0.5, 0.3));
    // let xx = x % self.w;
    // let yy = y % self.h;
    let xx = grid_fmod(x, self.w);
    let yy = grid_fmod(y, self.h);
    self.shape.dist(xx, yy)
  }
}

// Turn a distance into a color
fn _solid(d: f32) -> Pixel {
  let inside = d > 0.0;
  if inside { REDT } else { BLACK }
}

// Turn a distance into a color
const BANDWIDTH: f32 = 0.1;
fn band(d: f32) -> Pixel {
  let inside = d.abs() < (BANDWIDTH/2.0);
  if inside { REDT } else { NONE }
}

// TODO slow
// Ruler
const RULE_WIDTH: f32 = 0.05;
const SUB_RULE_WIDTH: f32 = 0.1;
const SUB_RULE_COUNT: f32 = 4.0;
fn ruler(d: f32) -> Pixel {
  let dist_from_unit = (d - d.floor()).abs();
  let rule = dist_from_unit < RULE_WIDTH;
  // let dist_from_sub_unit = ((d*SUB_RULE_COUNT) - ((d*SUB_RULE_COUNT).floor())).abs();
  // let sub_rule = dist_from_sub_unit < SUB_RULE_WIDTH;
  let inside = d < 0.0;
  if inside {
    REDT
  // } else if rule || sub_rule {
  } else if rule {
    BLACKT
  } else {
    NONE
  }
}

// fn compose<A, B, C, G, F>(f: F, g: G) -> impl Fn(A) -> C
// where
//     F: Fn(A) -> B,
//     G: Fn(B) -> C,
// {
//     move |x| g(f(x))
// }

fn main() {
    println!("Hello, world!");
  // let path = Path::new(r"image.png");
  // let file = File::create(path).unwrap();
  // let ref mut w = BufWriter::new(file);

  // let mut encoder = png::Encoder::new(w, 2, 2); // Width is 2 pixels and height is 1.
  // encoder.set_color(png::ColorType::Rgba);
  // encoder.set_depth(png::BitDepth::Eight);
  // encoder.set_trns(vec!(0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8));
  // encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
  // encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
  // let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
  //     (0.31270, 0.32900),
  //     (0.64000, 0.33000),
  //     (0.30000, 0.60000),
  //     (0.15000, 0.06000)
  // );
  // encoder.set_source_chromaticities(source_chromaticities);
  // let mut writer = encoder.write_header().unwrap();

  // let data = [255, 0, 0, 255, 0, 0, 0, 255,
  //             0, 255, 0, 255, 0, 0, 255, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
  // let data = fb.get_raw::<N>();

  // let mut fb: FB = FB::new(2, 2);
  // let red = Pixel { r: 255, g: 0, b: 0, a: 255 };
  // let green = Pixel { r: 0, g: 255, b: 0, a: 255 };
  // let blue = Pixel { r: 0, g: 0, b: 255, a: 255 };
  // let white = Pixel { r: 255, g: 255, b: 255, a: 255 };
  // fb.set(0, 0, &green);
  // fb.set(0, 1, &red);
  // fb.set(1, 0, &blue);
  // fb.set(1, 1, &white);
  // // fb.write("image.png".to_string());

  let (w, h) = (800, 800);
  // let (w, h) = (20, 20);
  // let (w, h) = (2, 2);
  let mut cfb = FB::new(w, h);
  let vd = 8.0;
  let view = Rect { ll: Pt { x: -vd, y: -vd }, ur: Pt { x: vd, y: vd } };
  let circle = Circle {};
  let moved_half = Translate::new(Box::new(circle), 0.5, 0.5);
  let ucircle = Translate::new(Box::new(Scale::new(Box::new(circle), 0.5, 0.5)), 1.0, 1.0);
  let moved = Translate::new(Box::new(circle), 1.0, 0.0);
  let moved2 = Translate::new(Box::new(circle), 1.0, 0.0);
  let moved3 = Translate::new(Box::new(circle), 1.0, 0.0);
  let moved4 = Translate::new(Box::new(circle), 1.0, 0.0);
  let moved5 = Translate::new(Box::new(circle), 1.0, 0.0);
  let inter = Intersection::new(Box::new(circle), Box::new(moved));
  let union = Union::new(Box::new(circle), Box::new(moved2));
  let diff = Difference::new(Box::new(circle), Box::new(moved4));
  let hmm = Hmm::new(Box::new(circle), Box::new(moved3));
  let smooth = SmoothUnion::new(Box::new(circle), Box::new(moved5));
  let grid = Grid::new(2.0, 2.0, Box::new(ucircle));
  render(&grid, ruler, view, &mut cfb);
  // render(&inter, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&union, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&hmm, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&diff, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&smooth, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  cfb.write("image.png".to_string());
}
