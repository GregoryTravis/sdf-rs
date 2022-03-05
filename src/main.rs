// TODO remove these
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate nalgebra as na;

use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use apng::Encoder;
use apng::Frame;
use apng::PNGImage;
use na::{Point3, Vector3};

const BLACK: Pixel = Pixel { r: 0, g: 0, b: 0, a: 255 };
const DGRAY: Pixel = Pixel { r: 64, g: 64, b: 64, a: 255 };
const GRAY: Pixel = Pixel { r: 128, g: 128, b: 128, a: 255 };
const LGRAY: Pixel = Pixel { r: 192, g: 192, b: 192, a: 255 };
const WHITE: Pixel = Pixel { r: 255, g: 255, b: 255, a: 255 };
const RED: Pixel = Pixel { r: 255, g: 0, b: 0, a: 255 };
const NONE: Pixel = Pixel { r: 0, g: 0, b: 0, a: 0 };

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

impl Pixel {
  pub fn mix(&self, op: Pixel) -> Pixel{
    Pixel {
      r: avg(self.r, op.r),
      g: avg(self.g, op.g),
      b: avg(self.b, op.b),
      a: avg(self.a, op.a),
    }
  }

  pub fn lerp(&self, p: Pixel, a: f32) -> Pixel {
    Pixel {
      r: ((self.r as f32 * (1.0 - a)) + (p.r as f32 * a)) as u8,
      g: ((self.g as f32 * (1.0 - a)) + (p.g as f32 * a)) as u8,
      b: ((self.b as f32 * (1.0 - a)) + (p.b as f32 * a)) as u8,
      a: ((self.a as f32 * (1.0 - a)) + (p.a as f32 * a)) as u8,
    }
  }
}

fn avgf32(a: f32, b: f32) -> f32 {
  (a + b) / 2.0
}

fn avg(a: u8, b: u8) -> u8 {
  ((a as f32 + b as f32) / 2.0) as u8
}

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
#[derive(Debug)]
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

  pub fn get(&self, x: usize, y: usize) -> Pixel {
    let off = ((y*self.w) + x) * 4;
    Pixel { r: self.pixels[off+0], b: self.pixels[off+1], g: self.pixels[off+2], a: self.pixels[off+3] }
  }

  pub fn set(&mut self, x: usize, y: usize, pix: &Pixel) {
    let off = ((y*self.w) + x) * 4;
    self.pixels[off+0] = pix.r;
    self.pixels[off+1] = pix.g;
    self.pixels[off+2] = pix.b;
    self.pixels[off+3] = pix.a;
  }

  pub fn blend_into(&mut self, x: usize, y: usize, pix: &Pixel) {
    let off = ((y*self.w) + x) * 4;
    // TODO unnecessary copy
    let current = Pixel { r: self.pixels[off+0], b: self.pixels[off+1], g: self.pixels[off+2], a: self.pixels[off+3] };
    let over = over(&pix, &current);
    self.set(x, y, &over);
  }

  pub fn write(&self, path: String) {
    // let path = Path::new(r"image.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, self.w.try_into().unwrap(), self.h.try_into().unwrap());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // encoder.set_trns(vec!(0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8));
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

    let sl: &[u8] = &self.pixels;
    writer.write_image_data(sl).unwrap(); // Save
  }
}

// Doesn't check that dims match
fn downsample_halve(fb: &FB, ofb: &mut FB) {
  for x in 0..ofb.w {
    for y in 0..ofb.h {
      let ox = x * 2;
      let oy = y * 2;
      let combined = fb.get(ox, oy).mix(fb.get(ox+1,oy)).mix(fb.get(ox, oy+1).mix(fb.get(ox+1, oy+1)));
      ofb.set(x, y, &combined)
    }
  }
}

fn upsample_render<S>(shape: &S,
                      colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
where
  S: Shape
{
  let mut ufb = FB::new(fb.w*2, fb.h*2);
  render(shape, colorer, domain, &mut ufb);
  downsample_halve(&ufb, fb);
}

fn render<S>(shape: &S, colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
where
  S: Shape
{
  let ox = domain.ll.x;
  let oy = domain.ll.y;
  let dx = (domain.ur.x - domain.ll.x) / (fb.w as f32);
  let dy = (domain.ur.y - domain.ll.y) / (fb.h as f32);
  for x in 0..fb.w {
    for y in 0..fb.h {
      let fx = ox + ((x as f32) * dx);
      let fy = oy + ((y as f32) * dy);
      fb.set(x, y, &colorer(shape, fx, fy));
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

#[derive(Clone, Copy, Debug)]
pub struct Square {
}

impl Shape for Square {
  fn dist(&self, x: f32, y: f32) -> f32 {
    (x.abs() - 1.0).max(y.abs() - 1.0)
  }
}

pub struct Translate {
  shape: Rc<dyn Shape>,
  tx: f32,
  ty: f32,
}

impl Translate {
  pub fn new(shape: Rc<dyn Shape>, tx: f32, ty: f32) -> Translate {
    Translate { shape: shape, tx: tx, ty: ty }
  }
}

impl Shape for Translate {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x - self.tx, y - self.ty)
  }
}

pub struct Scale {
  shape: Rc<dyn Shape>,
  sx: f32,
  sy: f32,
}

impl Scale {
  pub fn new(shape: Rc<dyn Shape>, sx: f32, sy: f32) -> Scale {
    Scale { shape: shape, sx: sx, sy: sy }
  }
}

impl Shape for Scale {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x / self.sx, y / self.sy)
  }
}

pub struct Union {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Union {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Union {
    Union { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Union {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).min(self.shape1.dist(x, y))
  }
}

pub struct Intersection {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Intersection {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Intersection {
    Intersection { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Intersection {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(self.shape1.dist(x, y))
  }
}

pub struct Difference {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Difference {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Difference {
    Difference { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Difference {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(-self.shape1.dist(x, y))
  }
}

pub struct Blend {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Blend {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Blend {
    Blend { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Blend {
  fn dist(&self, x: f32, y: f32) -> f32 {
    avgf32(self.shape0.dist(x, y), self.shape1.dist(x, y))
  }
}

pub struct SmoothUnion {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl SmoothUnion {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> SmoothUnion {
    SmoothUnion { shape0: shape0, shape1: shape1 }
  }
}

// From https://www.ronja-tutorials.com/post/035-2d-sdf-combination/
impl Shape for SmoothUnion {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let r = 0.3;
    let d0 = self.shape0.dist(x, y);
    let d1 = self.shape1.dist(x, y);
    let md0 = (d0 - r).min(0.0);
    let md1 = (d1 - r).min(0.0);
    let inside_distance = -length(md0, md1);
    let simple_union = d0.min(d1);
    let outside_distance = simple_union.max(r);
    inside_distance + outside_distance
  }
}

pub struct Hmm {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Hmm {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Hmm {
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
  shape: Rc<dyn Shape>,
}

impl Grid {
  pub fn new(w: f32, h: f32, shape: Rc<dyn Shape>) -> Grid {
    Grid { w: w, h: h, shape: shape }
  }
}

fn grid_fmod(a: f32, b: f32) -> f32 {
  let aob = a / b;
  (aob - aob.floor()) * b
}

impl Shape for Grid {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let xx = grid_fmod(x, self.w);
    let yy = grid_fmod(y, self.h);
    self.shape.dist(xx, yy)
  }
}

// Turn a distance into a color
fn _solid(d: f32) -> Pixel {
  let inside = d > 0.0;
  if inside { RED } else { BLACK }
}

// Turn a distance into a color
const BANDWIDTH: f32 = 0.1;
fn band(d: f32) -> Pixel {
  let inside = d.abs() < (BANDWIDTH/2.0);
  if inside { RED } else { NONE }
}

// Like bump_map but just the edges
// Assumes d <= 0
const BEVEL_WIDTH: f32 = 0.075;
fn bevel_dist_to_ht(d: f32) -> f32 {
  let pi = std::f32::consts::PI;
  if d < -BEVEL_WIDTH {
    1.0
  } else {
    let sd = d / BEVEL_WIDTH;
    ((pi/2.0) * (sd+1.0)).cos()
  }
}

//   c
// a-+-b
//   d
fn bevel<S>(shape: &S, x: f32, y:f32) -> Pixel
where
  S: Shape
{
  let dist = shape.dist(x, y);

  if dist > 0.0 {
    return BLACK;
  }

  let bit = 0.005;
  let ax = x - bit;
  let ay = y;
  let bx = x + bit;
  let by = y;
  let cx = x;
  let cy = y - bit;
  let dx = x;
  let dy = y + bit;

  let adist = shape.dist(ax, ay);
  let bdist = shape.dist(bx, by);
  let cdist = shape.dist(cx, cy);
  let ddist = shape.dist(dx, dy);

  let az = bevel_dist_to_ht(adist);
  let bz = bevel_dist_to_ht(bdist);
  let cz = bevel_dist_to_ht(cdist);
  let dz = bevel_dist_to_ht(ddist);

  let a = Point3::new(ax, ay, az);
  let b = Point3::new(bx, by, bz);
  let c = Point3::new(cx, cy, cz);
  let d = Point3::new(dx, dy, dz);

  let ba: Vector3<f32> = b - a;
  let cd: Vector3<f32> = c - d;

  let norm = cd.cross(&ba).normalize();

  let light = Vector3::new(-1.0, 1.0, 1.0).normalize();

  let brightness = norm.dot(&light);

  GRAY.lerp(WHITE, brightness.clamp(0.0, 1.0))
}

// TODO slow
// Ruler
const RULE_SEP: f32 = 4.0;
const RULE_WIDTH: f32 = 0.05;
const SUB_RULE_WIDTH: f32 = 0.1;
const SUB_RULE_COUNT: f32 = 4.0;
fn ruler<S>(shape: &S, x: f32, y:f32) -> Pixel
where
  S: Shape
{
  let d = shape.dist(x, y);
  let dist_from_unit = ((d*RULE_SEP) - (d*RULE_SEP).floor()).abs() / RULE_SEP;
  let rule = dist_from_unit < RULE_WIDTH;
  let inside = d < 0.0;
  if inside {
    RED
  } else if rule {
    BLACK
  } else {
    NONE
  }
}

fn main() {
    println!("Hello, world!");
  let (w, h) = (800, 800);
  let vd = 4.0;
  let view = Rect { ll: Pt { x: -vd, y: -vd }, ur: Pt { x: vd, y: vd } };

  // cfb.write("image.png".to_string());
  let mut files = Vec::new();
  let num_frames = 10;
  for x in 0..num_frames {
    let mut acfb = FB::new(w, h);
    let filename = format!("image{:0>10}.png", x);
    let dt = (x as f32) / 40.0;
    let s = wacky2(dt);
    let start = Instant::now();
    upsample_render(&s, bevel, view, &mut acfb);
    eprintln!("elapsed {:?}", start.elapsed()); // note :?
    acfb.write(filename.clone());
    files.push(filename);
  }

  let mut png_images: Vec<PNGImage> = Vec::new();
  for f in files.iter() {
      png_images.push(apng::load_png(f).unwrap());
  }

  let path = Path::new(r"anim.png");
  let mut out = BufWriter::new(File::create(path).unwrap());

  let config = apng::create_config(&png_images, None).unwrap();
  let mut encoder = Encoder::new(&mut out, config).unwrap();
  let frame = Frame {
      delay_num: Some(1),
      delay_den: Some(20),
      ..Default::default()
  };

  match encoder.encode_all(png_images, Some(&frame)) {
      Ok(_n) => println!("success"),
      Err(err) => eprintln!("{}", err),
  }

  let clean_up = true;
  if clean_up {
      for filename in files {
          match std::fs::remove_file(filename) {
              Ok(_n) => (),
              Err(err) => eprintln!("{}", err),
          }
      }
  }
}

fn just_circle(_t: f32) -> impl Shape {
  Circle {}
}

fn cgrid_circle(_t: f32) -> impl Shape {
  let sc = Circle {};
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(sc), 0.5, 0.5)), 1.0, 1.0));
  let grid = Grid::new(2.0, 2.0, ucircle.clone());
  grid
}

fn blah(_t: f32) -> impl Shape {
  let s = Square{};
  let c = Circle{};
  Blend::new(Rc::new(c), Rc::new(s))
}

fn blend(_t: f32) -> impl Shape {
  let c = Rc::new(Circle {});
  let s = Rc::new(Square {});
  let c0 = Translate::new(c.clone(), -0.90, 0.0);
  let c1 = Translate::new(s.clone(),  0.75, 0.0);
  let all = SmoothUnion::new(Rc::new(c0), Rc::new(c1));
  // let all = Union::new(Rc::new(c0), Rc::new(c1));
  all
}

fn wacky2(t: f32) -> impl Shape {
  // let circle = Circle {};
  let s = Square{};
  let c = Circle{};
  let sc = Blend::new(Rc::new(c), Rc::new(s));

  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(sc), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = SmoothUnion::new(Rc::new(Translate::new(grid.clone(), 0.2 + t, 0.2)), Rc::new(Translate::new(grid2.clone(), 0.2, 0.2 + t)));
  gridi
}

fn wacky(t: f32) -> (impl Shape, impl Shape) {
  let circle = Circle {};
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(circle), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = Intersection::new(grid.clone(), grid2.clone());
  let gridu = Translate::new(Rc::new(Union::new(grid.clone(), grid2.clone())), 0.2, 0.2 + t);
  (gridi, gridu)
}
