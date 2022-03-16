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
use na::{Point3, Vector3, Vector2};
use rand::Rng;

const BLACK: Pixel = Pixel { r: 0.0, g: 0.0, b: 0.0, a: 255.0 };
const DGRAY: Pixel = Pixel { r: 64.0, g: 64.0, b: 64.0, a: 255.0 };
const GRAY: Pixel = Pixel { r: 128.0, g: 128.0, b: 128.0, a: 255.0 };
const LGRAY: Pixel = Pixel { r: 192.0, g: 192.0, b: 192.0, a: 255.0 };
const WHITE: Pixel = Pixel { r: 255.0, g: 255.0, b: 255.0, a: 255.0 };
const RED: Pixel = Pixel { r: 255.0, g: 0.0, b: 0.0, a: 255.0 };
const NONE: Pixel = Pixel { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

const OLD: bool = false;
const UPSAMPLE_RENDER: bool = false;

pub fn length(a: f32, b: f32) -> f32 {
  (a*a + b*b).sqrt()
}

#[derive(Clone, Copy, Debug)]
pub struct Pixel {
  pub r: f32,
  pub g: f32,
  pub b: f32,
  pub a: f32,
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
      r: (self.r * (1.0 - a)) + (p.r * a),
      g: (self.g * (1.0 - a)) + (p.g * a),
      b: (self.b * (1.0 - a)) + (p.b * a),
      a: (self.a * (1.0 - a)) + (p.a * a),
    }
  }
}

fn avg(a: f32, b: f32) -> f32 {
  (a + b) / 2.0
}

// fn avg(a: u8, b: u8) -> u8 {
//   ((a as f32 + b as f32) / 2.0) as u8
// }

pub fn over(a: &Pixel, b: &Pixel) -> Pixel {
  let ao = (a.a + (b.a * (255.0 - a.a))).clamp(0.0, 255.0);
  let over = Pixel {
    a: ao,
    r: (((a.r * a.a) + (b.r * b.a * (255.0 - a.a))) / ao).clamp(0.0, 255.0),
    g: (((a.g * a.a) + (b.g * b.a * (255.0 - a.a))) / ao).clamp(0.0, 255.0),
    b: (((a.b * a.a) + (b.b * b.a * (255.0 - a.a))) / ao).clamp(0.0, 255.0),
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
  pixels: Vec<Pixel>,
}

impl FB {
  pub fn new(w: usize, h: usize) -> FB {
    let fb = FB {
      w: w,
      h: h,
      pixels: vec![NONE; w*h],
    };
    return fb;
  }

  pub fn get(&self, x: usize, y: usize) -> Pixel {
    let off = (y*self.w) + x;
    self.pixels[off]
  }

  pub fn set(&mut self, x: usize, y: usize, pix: &Pixel) {
    let off = (y*self.w) + x;
    self.pixels[off] = *pix;
  }

  pub fn blend_into(&mut self, x: usize, y: usize, pix: &Pixel) {
    let current = self.get(x, y);
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

    let mut image_data = Vec::with_capacity(self.w * self.h * 4);
    for (i, x) in self.pixels.iter().enumerate() {
      image_data.push((*x).r as u8);
      image_data.push((*x).g as u8);
      image_data.push((*x).b as u8);
      image_data.push((*x).a as u8);
    }
    let sl: &[u8] = &image_data;
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

fn upsample_render(shape: Rc<dyn Shape>,
                   colorer: fn(shape: Rc<dyn Shape>, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
{
  let mut ufb = FB::new(fb.w*2, fb.h*2);
  render(shape, colorer, domain, &mut ufb);
  downsample_halve(&ufb, fb);
}

fn render(shape: Rc<dyn Shape>, colorer: fn(shape: Rc<dyn Shape>, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
// fn render<S>(shape: &S, colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
// where
//   S: Shape
{
  let ox = domain.ll.x;
  let oy = domain.ll.y;
  let dx = (domain.ur.x - domain.ll.x) / (fb.w as f32);
  let dy = (domain.ur.y - domain.ll.y) / (fb.h as f32);
  for x in 0..fb.w {
    for y in 0..fb.h {
      let fx = ox + ((x as f32) * dx);
      let fy = oy + ((y as f32) * dy);
      fb.set(x, y, &colorer(shape.clone(), fx, fy));
    }
  }
}

pub trait Shape: std::fmt::Debug {
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

fn non_stupid_atan2(x: f32, y: f32) -> f32 {
  let mut a = y.atan2(x);
  if a < 0.0 {
    a += 2.0 * std::f32::consts::PI;
  }
  a
}

#[derive(Clone, Copy, Debug)]
pub struct Flower {
  num_petals: i32,
}

impl Flower {
  pub fn new(num_petals: i32) -> Flower {
    Flower { num_petals: num_petals }
  }
}

impl Shape for Flower {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let ang = non_stupid_atan2(x, y);
    let raw_dist = (x*x + y*y).sqrt();
    let radius = (ang * (self.num_petals as f32 / 2.0)).sin().abs();
    let dist = raw_dist / radius;
    // println!("flower {} {} {} {} {} {}", x, y, ang, raw_dist, radius, dist);
    dist - 1.0
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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
    avg(self.shape0.dist(x, y), self.shape1.dist(x, y))
  }
}

#[derive(Debug)]
pub struct Interp {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
  alpha: f32,
}

impl Interp {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>, alpha: f32) -> Interp {
    Interp { shape0: shape0, shape1: shape1, alpha: alpha }
  }
}

impl Shape for Interp {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let d0 = self.shape0.dist(x, y);
    let d1 = self.shape1.dist(x, y);
    (d0 * self.alpha) + (d1 * (1.0 - self.alpha))
  }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Rotation {
  shape: Rc<dyn Shape>,
  bx: Vector2<f32>,
  by: Vector2<f32>,
}

impl Rotation {
  pub fn new(shape: Rc<dyn Shape>, ang: f32) -> Rotation {
    let bx = Vector2::new(ang.cos(), ang.sin());
    let by = Vector2::new(-ang.sin(), ang.cos());
    Rotation { shape: shape, bx: bx, by: by }
  }
}

impl Shape for Rotation {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let rv = x * self.bx + y * self.by;
    self.shape.dist(rv.x, rv.y)
  }
}

#[derive(Debug)]
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

// Returns (mod, i), I forget what they're called
fn grid_fmod2(a: f32, b: f32) -> (f32, i32) {
  let aob = a / b;
  (((aob - aob.floor()) * b), aob.floor() as i32)
}

impl Shape for Grid {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let xx = grid_fmod(x, self.w);
    let yy = grid_fmod(y, self.h);
    self.shape.dist(xx, yy)
  }
}

#[derive(Debug)]
pub struct ParityFlipGrid {
  w: f32,
  h: f32,
  shape: Rc<dyn Shape>,
}

impl ParityFlipGrid {
  pub fn new(w: f32, h: f32, shape: Rc<dyn Shape>) -> ParityFlipGrid {
    ParityFlipGrid { w: w, h: h, shape: shape }
  }
}

impl Shape for ParityFlipGrid {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let (mut xx, xi) = grid_fmod2(x, self.w);
    let (mut yy, yi) = grid_fmod2(y, self.h);
    if xi.abs() % 2 == 1 {
      xx = self.w - xx;
    }
    if yi.abs() % 2 == 1 {
      yy = self.h - yy;
    }
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
fn bevel(shape: Rc<dyn Shape>, x: f32, y:f32) -> Pixel
{
  let dist = shape.dist(x, y);
  // println!("dist {} {} {}", x, y, dist);

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

fn compile_animation(files: &Vec<String>, ofile: &str) {
  let mut png_images: Vec<PNGImage> = Vec::new();
  for f in files.iter() {
    png_images.push(apng::load_png(f).unwrap());
  }

  let path = Path::new(ofile);
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
}

fn main() {
  if OLD {
    old_main();
  } else {
    shp_main();
  }
}

fn old_main() {
  println!("Hello, world!");
  // println!("{} {} {} {}", 4.0_f32.floor(), 4.3_f32.floor(), -4.0_f32.floor(), -4.3_f32.floor());
  // println!("{} {} {} {}", 4.0_f32.floor() as i32, 4.3_f32.floor() as i32, -4.0_f32.floor() as i32, -4.3_f32.floor() as i32);
  // println!("{:?} {:?} {:?} {:?}", grid_fmod2(4.0_f32, 3.0), grid_fmod2(4.3_f32, 3.0), grid_fmod2(-4.0_f32, 3.0), grid_fmod2(-4.3_f32, 3.0));
  println!("{} {} {} {}",
           1.0_f32.atan2(1.0), // 1st
           1.0_f32.atan2(-1.0), // 2nd
           -1.0_f32.atan2(-1.0), // 3rd
           -1.0_f32.atan2(1.0)); // 4th
  println!("{} {} {} {} {} {} {} {}",
           non_stupid_atan2(1.0, 0.0),
           non_stupid_atan2(1.0, 1.0),
           non_stupid_atan2(0.0, 1.0),
           non_stupid_atan2(-1.0, 1.0),
           non_stupid_atan2(-1.0, 0.0),
           non_stupid_atan2(-1.0, -1.0),
           non_stupid_atan2(0.0, -1.0),
           non_stupid_atan2(1.0, -1.0));
  let (w, h) = (800, 800);
  // let (w, h) = (4, 4);
  let vd = 2.0;
  let view = Rect { ll: Pt { x: -vd, y: -vd }, ur: Pt { x: vd, y: vd } };
  let num_frames = 1;

  // render_animation_to(w, h, view, num_frames, wacky6, bevel, r"anim.png");

  // let s = rand_shape();
  let s = wacky8(0.0);
  println!("{:?}", s);
  render_animation_to(w, h, view, num_frames, Rc::new(s), bevel, r"anim.png");
}

fn render_animation_to(w: usize, h: usize, view: Rect<f32>, num_frames: u32,
                          // sf: fn(f32) -> S, colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, ofile: &str)
                          // sf: fn(f32) -> Rc<dyn Shape>, colorer: impl Fn(&S, f32, f32) -> Pixel, ofile: &str)
                          s: Rc<dyn Shape>, colorer: fn(Rc<dyn Shape>, f32, f32) -> Pixel, ofile: &str)
// where S: Shape
{
  // cfb.write("image.png".to_string());
  let mut files = Vec::new();
  for x in 0..num_frames {
    let mut acfb = FB::new(w, h);
    let filename = format!("image{:0>10}.png", x);
    let dt = (x as f32) / 40.0;
    // let s = sf(dt);
    let start = Instant::now();
    if UPSAMPLE_RENDER {
      upsample_render(s.clone(), colorer, view, &mut acfb);
    } else {
      render(s.clone(), colorer, view, &mut acfb);
    }
    eprintln!("elapsed {:?}", start.elapsed()); // note :?
    acfb.write(filename.clone());
    files.push(filename);
  }

  compile_animation(&files, r"anim.png");

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

fn wacky3(t: f32) -> impl Shape {
  // let circle = Circle {};
  let s = Square{};
  let c = Circle{};
  let sc = Blend::new(Rc::new(c), Rc::new(s));

  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(sc), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = SmoothUnion::new(Rc::new(Rotation::new(Rc::new(Translate::new(grid.clone(), 0.2 + t, 0.2)), t)), Rc::new(Translate::new(grid2.clone(), 0.2, 0.2 + t)));
  gridi
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

fn wacky4(t: f32) -> impl Shape {
  let circle = Circle {};
  let ucircle2 = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(circle), 0.5, 0.5)), 1.0, 1.0));
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(wacky5(t*10.0)), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Translate::new(Rc::new(Grid::new(2.0, 2.0, ucircle.clone())), 0.0, t));
  let grid2 = Rc::new(Rotation::new(Rc::new(Grid::new(2.0, 2.0, ucircle2.clone())), t));
  let grid3 = Rc::new(Translate::new(Rc::new(Scale::new(grid.clone(), 2.5, 2.5)), t, 0.0));
  let gridi = Difference::new(grid3.clone(), grid2.clone());
  gridi
}

fn wacky5(t: f32) -> impl Shape {
  let circle = Rc::new(Circle {});
  let ucircle = Rc::new(Translate::new(
    Rc::new(Scale::new(
      Rc::new(Rotation::new(
        Rc::new(Square {}), t)), 0.5, 0.5)), 1.0, 1.0));
  let gridi = SmoothUnion::new(circle, ucircle);
  gridi
}

fn wacky6(t: f32) -> impl Shape {
  let circle = Circle {};
  let ucircle2 = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(circle), 0.5, 0.5)), 1.0, 1.0));
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(wacky5(t*10.0)), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Translate::new(Rc::new(Grid::new(2.0, 2.0, ucircle.clone())), 0.0, t));
  let grid2 = Rc::new(Rotation::new(Rc::new(Grid::new(2.0, 2.0, ucircle2.clone())), t));
  let grid3 = Rc::new(Translate::new(Rc::new(Scale::new(grid.clone(), 2.5, 2.5)), t, 0.0));
  let alpha = t.sin();
  let gridi = Interp::new(grid3.clone(), grid2.clone(), alpha);
  gridi
}

fn wacky7(t: f32) -> impl Shape {
  let circle = Rc::new(Translate::new(Rc::new(Circle {}), -0.25, -0.25));
  let grid = ParityFlipGrid::new(2.0, 2.0, circle.clone());
  grid
}

fn wacky8(t: f32) -> impl Shape {
  Flower::new(7)
}

// fn randFromVec<T>(vec: &Vec<T>) -> T {
//   let n: f32 = rand::thread_rng().gen();
//   let i: usize = (n * vec.length()) as usize;
//   vec[i]
// }

// // Takes two shapes present in the unit square, grids them and slowly
// // slides/translates them, with smooth union.
// fn grid_grind(t: f32) -> impl Shape {
// }

fn rand_atom() -> Rc<dyn Shape> {
  let n: f32 = rand::thread_rng().gen();
  if n < 0.5 {
    Rc::new(Circle {})
  } else {
    Rc::new(Square {})
  }
}

// fn rand_unop<S>(s: &S)-> Box<dyn Shape>
fn rand_unop(s: Rc<dyn Shape>)-> Rc<dyn Shape>
// where S: Shape
{
  let n: f32 = rand::thread_rng().gen();
  let fcount = 4.0;
  if n < 1.0/fcount {
    let sx: f32 = rand::thread_rng().gen::<f32>() * 2.0;
    let sy: f32 = rand::thread_rng().gen::<f32>() * 2.0;
    Rc::new(Scale::new(s, sx, sy))
  } else if n < 2.0/fcount {
    let tx: f32 = rand::thread_rng().gen::<f32>() * 3.0;
    let ty: f32 = rand::thread_rng().gen::<f32>() * 3.0;
    Rc::new(Translate::new(s, tx, ty))
  } else if n < 3.0/fcount {
    let w: f32 = rand::thread_rng().gen::<f32>() + 1.0;
    let h: f32 = rand::thread_rng().gen::<f32>() + 1.0;
    Rc::new(Grid::new(w, h, s))
  } else {
    let ang: f32 = rand::thread_rng().gen::<f32>() * std::f32::consts::PI;
    Rc::new(Rotation::new(s, ang))
  }
}

fn rand_binop(s0: Rc<dyn Shape>, s1: Rc<dyn Shape>) -> Rc<dyn Shape> {
  let n: f32 = rand::thread_rng().gen();
  let fcount = 6.0;
  if n < 1.0/fcount {
    Rc::new(Union::new(s0, s1))
  } else if n < 2.0/fcount {
    Rc::new(Difference::new(s0, s1))
  } else if n < 3.0/fcount {
    Rc::new(Intersection::new(s0, s1))
  } else if n < 4.0/fcount {
    Rc::new(Blend::new(s0, s1))
  } else if n < 5.0/fcount {
    let alpha: f32 = rand::thread_rng().gen();
    Rc::new(Interp::new(s0, s1, alpha))
  } else {
    Rc::new(SmoothUnion::new(s0, s1))
  }
}

fn rand_shape() -> Rc<dyn Shape> {
  let n: f32 = rand::thread_rng().gen();
  let fcount = 3.0;
  if n < 1.0/fcount {
    rand_atom()
  } else if n < 2.0/fcount {
    rand_unop(rand_shape())
  } else {
    rand_binop(rand_shape(), rand_shape())
  }
}

// enum Atom {
//   Fun(Box<dyn Fn(f32, f32, f32) -> f32>),
// }

type Shp = Rc<dyn Fn(f32, f32, f32) -> f32>;
type Colorer = Rc<dyn Fn(Shp, f32, f32, f32) -> Pixel>;

fn render_shp(shape: Shp, colorer: Colorer, domain: Rect<f32>, fb: &mut FB, t: f32)
{
  let ox = domain.ll.x;
  let oy = domain.ll.y;
  let dx = (domain.ur.x - domain.ll.x) / (fb.w as f32);
  let dy = (domain.ur.y - domain.ll.y) / (fb.h as f32);
  for x in 0..fb.w {
    for y_inv in 0..fb.h {
      let y = fb.h - 1 - y_inv;
      let fx = ox + ((x as f32) * dx);
      let fy = oy + ((y as f32) * dy);
      fb.set(x, y, &colorer(shape.clone(), fx, fy, t));
    }
  }
}

fn render_shp_to(w: usize, h: usize, view: Rect<f32>, num_frames: u32,
                 s: Shp, colorer: Colorer, ofile: &str)
{
  let mut files = Vec::new();
  for x in 0..num_frames {
    let mut acfb = FB::new(w, h);
    let filename = format!("image{:0>10}.png", x);
    let t = (x as f32) / 40.0;
    let start = Instant::now();
    render_shp(s.clone(), colorer.clone(), view, &mut acfb, t);
    eprintln!("elapsed {:?}", start.elapsed()); // note :?
    acfb.write(filename.clone());
    files.push(filename);
  }

  compile_animation(&files, r"anim.png");

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

fn circle() -> Shp { Rc::new(|x: f32, y: f32, t: f32| { length(x, y) - 1.0 }) }
fn square() -> Shp { Rc::new(|x: f32, y: f32, t: f32| { (x.abs() - 1.0).max(y.abs() - 1.0) }) }

type Transform = Rc<dyn Fn(f32, f32, f32) -> (f32, f32, f32)>;

type DistBinop = Rc<dyn Fn(f32, f32) -> f32>;

fn translate(start_tx: f32, start_ty: f32, delta_tx: f32, delta_ty: f32) -> Transform {
  Rc::new(move |x: f32, y:f32, t: f32| {
    let tx = start_tx + t * delta_tx;
    let ty = start_ty + t * delta_ty;
    (x - tx, y - ty, t)
  })
}

fn rotation(start_ang: f32, delta_ang: f32) -> Transform {
  Rc::new(move |x: f32, y:f32, t: f32| {
    let ang = start_ang + t * delta_ang;
    let bx = Vector2::new(ang.cos(), ang.sin());
    let by = Vector2::new(-ang.sin(), ang.cos());
    let rv = x * bx + y * by;
    (rv.x, rv.y, t)
  })
}

fn transform(s: Shp, tr: Transform) -> Shp {
  Rc::new(move |x: f32, y: f32, t: f32| {
    let (nx, ny, nt) = tr(x, y, t);
    s(nx, ny, nt)
  })
}

fn smooth_union(s0: Shp, s1: Shp) -> Shp {
  binopper(s0, s1, Rc::new(smooth_union_binop))
}

fn binopper(s0: Shp, s1: Shp, op: DistBinop) -> Shp {
  Rc::new(move |x: f32, y: f32, t: f32| {
    let d0 = s0(x, y, t);
    let d1 = s1(x, y, t);
    op(d0, d1)
  })
}

fn smooth_union_binop(d0: f32, d1: f32) -> f32 {
  let r = 0.3;
  let md0 = (d0 - r).min(0.0);
  let md1 = (d1 - r).min(0.0);
  let inside_distance = -length(md0, md1);
  let simple_union = d0.min(d1);
  let outside_distance = simple_union.max(r);
  inside_distance + outside_distance
}

fn shp_main() {
  let (w, h) = (800, 800);
  // let (w, h) = (4, 4);
  let vd = 2.0;
  let view = Rect { ll: Pt { x: -vd, y: -vd }, ur: Pt { x: vd, y: vd } };
  let num_frames = 50;

  // let s = |x: f32, y: f32, t: f32| { length(x, y) - 1.0 };
  // let s = square();
  let s = smooth_union(
    transform(transform(square(), translate(0.0, 0.0, 1.0, 0.0)), rotation(0.0, 0.7)),
    transform(circle(), translate(0.0, 0.0, 0.0, 2.0)));
  render_shp_to(w, h, view, num_frames, s, Rc::new(bevel_shp), r"anim.png");
}

fn bevel_shp(shape: Shp, x: f32, y:f32, t: f32) -> Pixel
{
  let dist = shape(x, y, t);
  // println!("dist {} {} {}", x, y, dist);

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

  let adist = shape(ax, ay, t);
  let bdist = shape(bx, by, t);
  let cdist = shape(cx, cy, t);
  let ddist = shape(dx, dy, t);

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

// #[derive(Debug)]
// pub struct ShpShape {
//   shp: Shp,
// }

// impl Shape for ShpShape {
//   fn dist(&self, x: f32, y: f32) -> f32 {
//     self.shape.dist(x - self.tx, y - self.ty)
//   }
// }

// fn shp_to_shape(shp: Shp) -> impl Shape {
//   ShpShape { shp: shp }
// }
