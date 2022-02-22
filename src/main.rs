use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;
// use std::path::Path;

#[derive(Clone, Copy)]
pub struct Pixel {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

#[allow(unused_variables, dead_code)]
pub struct FB {
  w: usize,
  h: usize,
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
    self.pixels[off+0] = pix.r;
    self.pixels[off+1] = pix.g;
    self.pixels[off+2] = pix.b;
    self.pixels[off+3] = pix.a;
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

  let mut fb: FB = FB::new(2, 2);
  let red = Pixel { r: 255, g: 0, b: 0, a: 255 };
  let green = Pixel { r: 0, g: 255, b: 0, a: 255 };
  let blue = Pixel { r: 0, g: 0, b: 255, a: 255 };
  let white = Pixel { r: 255, g: 255, b: 255, a: 255 };
  fb.set(0, 0, &green);
  fb.set(0, 1, &red);
  fb.set(1, 0, &blue);
  fb.set(1, 1, &white);
  fb.write("image.png".to_string());

}