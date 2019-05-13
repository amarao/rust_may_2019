#![allow(dead_code)]
extern crate minifb;
extern crate clipboard;
extern crate lodepng;

use itertools::Itertools;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::cmp;
use minifb::{Key, WindowOptions, Window};
use std::f32::consts::*;

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;

use lodepng::encode32 as png_encode;


type Float = f32;
fn sin(x:Float) ->Float {
    x.sin()
}
fn cos(x:Float) -> Float {
    x.cos()
}

#[derive (PartialEq)]
#[derive (Clone)]
struct Pixel {
    index: usize,
}

impl  Pixel {
    fn as_pixel(&self, canvas: &Canvas) -> [i64; 2]{
        let row = self.index as i64 / canvas.pixel_x as i64 - canvas.zero_y as i64;
        let column = self.index as i64 %  canvas.pixel_x  as i64 - canvas.zero_x as i64;
        [row, column]
    }
    fn as_cartesian(&self, canvas: &Canvas) -> [Float; 2]{
        let [row, column] = self.as_pixel(canvas);
        let y = (row as Float) * canvas.pixel_size_y;
        let x = (column as Float) * canvas.pixel_size_x;
        [x,  y]
    }
    fn iterate_lattice_as_cartesian(&self, canvas: &Canvas, lattice_dim:usize) -> impl Iterator<Item =[Float;2]> {
        let [x,y] = self.as_cartesian(canvas);
        let (dx, dy) = (canvas.pixel_size_x, canvas.pixel_size_y);
        let subcanvas = (lattice_dim - 1) as Float;
        let conv = move |(i, j): (usize, usize)| {
                [
                    x+dx/subcanvas * i as Float,
                    y+dy/subcanvas * j as Float
                ]
            };
        (0..lattice_dim).cartesian_product(0..lattice_dim).map(conv)
    }

    fn sign_change_on_lattice<F> (&self, func:F, canvas: &Canvas, lattice_dim:usize) -> bool where
        F: Fn(Float, Float) -> Float
    {
        let mut sign: Option<bool> = None;
        for [x, y] in self.iterate_lattice_as_cartesian(canvas, lattice_dim){
            let res = func(x,y);
            if !res.is_finite() {return false};
            let num_sign = res.signum() > 0.0;
            sign = match sign {
                None => {Some(num_sign)},
                Some(old_sign) if old_sign != num_sign => { return true },
                _ => {continue}
            };
        }
        false
    }
}

type PixelColor = u8;
struct Canvas {
    img: Vec<PixelColor>,
    pixel_x: usize,
    pixel_y: usize,
    pixel_size_x: Float,
    pixel_size_y: Float,
    zero_x: usize,
    zero_y: usize,
}

impl Canvas{
    fn new(canvas_x: usize, canvas_y:usize, cartesian_x: Float, cartesian_y:Float, zero_position_x: usize, zero_position_y: usize) -> Canvas {
        let img_size = ((canvas_x as u64)*(canvas_y as u64)) as usize;
        let canvas = Canvas{
                img: vec![0xFF;img_size],
                pixel_x: canvas_x,
                pixel_y: canvas_y,
                zero_x: zero_position_x,
                zero_y: zero_position_y,
                pixel_size_x: cartesian_x/(canvas_x as Float),
                pixel_size_y: cartesian_y/(canvas_y as Float),
        };
        canvas
    }
    fn iter(&self) ->  impl Iterator<Item = Pixel>{
        (0..(self.pixel_x*self.pixel_y)).into_iter().map(|x|Pixel{index: x as usize})
    }


    fn get_neighbors(& self, pixel: & Pixel) -> Vec<Pixel>{
        let mut res: Vec<Pixel> = Vec::with_capacity(8);
        for x in -10..11{
            for y in -10..11 {
                if x==y && y==0 { continue };
                let neighbor = pixel.index as i64 + y as i64 *self.pixel_x as i64 + x as i64;
                if neighbor < 0 || neighbor >= self.pixel_x as i64 *self.pixel_y as i64{
                    continue;
                }
                res.push(Pixel{index: neighbor as usize});
            }
        }
        res
    }
    fn neighbors_roots_count(&self, pixel: &Pixel) -> u64 {
        let mut res = 0;
        for x in -30..31{
            for y in -30..31 {
                if x==y && y==0 { continue };
                let neighbor = pixel.index as i64 + y as i64 *self.pixel_x as i64 + x as i64;
                if neighbor < 0 || neighbor >= self.pixel_x as i64 *self.pixel_y as i64{
                    continue;
                }
                if self.img[neighbor as usize] == 0 {res += 1;}
            }
    }
    res
}

    fn set_pixel(& mut self, pixel: &Pixel, value: PixelColor) {
        self.img[pixel.index] = value;
    }
    fn get_pixel(&self, pixel:&Pixel) -> PixelColor {
        self.img[pixel.index]
    }
    fn roots(&self) -> u64{
        self.img.iter().filter(|&x| *x==0).count() as u64
    }
}

fn copy_to_clipboard(canvas: &Canvas){
    // let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    // ctx.set_contents(&canvas.img);
    println!("encoding start");
    let png = png_encode(&canvas.img, canvas.pixel_x, canvas.pixel_y).unwrap();
    // use std::mem::size_of_val;
    println!("encoding end, size: {}", png.len());
    println!("Copying to clipboard");
}

fn show_and_wait(canvas:Canvas){
    let mut window = Window::new("Test - ESC to exit",
                                 canvas.pixel_x,
                                 canvas.pixel_y,
                                 WindowOptions::default()
                                 ).unwrap();
    let new_img: Vec<u32> = canvas.img.iter().map(|x| (*x as u32)*0x0101_0101).collect();
    std::thread::sleep(Duration::new(0,150_000_000));
    window.update_with_buffer(&new_img).unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_down(Key::Enter){
            copy_to_clipboard(&canvas)
        }
        let start = Instant::now();
        window.update();
        let spend = start.elapsed();
        sleep(Duration::new(0,1000000000/60).checked_sub(spend).unwrap_or(Duration::new(0,0)));
    }
}

fn render<F>(canvas: &mut Canvas, f: &F, lattice_dim: usize) where
    F: Fn(Float, Float) -> Float
{
    for pixel in canvas.iter(){
        if pixel.sign_change_on_lattice(f, &canvas, lattice_dim){
            canvas.set_pixel(&pixel, 0);
        }
    }
}

fn up_render<F>(canvas: &mut Canvas, f: &F, lattice_dim:usize) where
    F: Fn(Float, Float) -> Float
{
    for pixel in canvas.iter(){
        if canvas.img[pixel.index]!=0 as PixelColor {
            if pixel.sign_change_on_lattice(f, &canvas, lattice_dim){
                canvas.set_pixel(&pixel, 0);
            }
        }
    }
}


fn clarify<F>(canvas: &mut Canvas, f: &F, lattice_dim:usize) -> u64 where
    F: Fn(Float, Float) -> Float
{
    let mut update_count = -1;
    let mut iteration = 0;
    let mut scandepth: Vec<u16> = vec![lattice_dim as u16;canvas.img.len()];
    for _ in 0..6 {
        let mut last_roots: Vec<Pixel> = Vec::new();
        while update_count !=0  {
            update_count = 0;
            iteration += 1;
            let mut max_boost = 1;
            let scan_lattice = (lattice_dim as u64 + iteration) as usize;
            let deepscan_lattice = (lattice_dim as u64 + iteration*2) as usize;
            let mut pix_count = 0;
            for pixel in canvas.iter(){
                if canvas.get_pixel(&pixel) != 0 {
                    if scandepth[pixel.index] < scan_lattice as u16{
                        scandepth[pixel.index] = scan_lattice as u16;
                        if canvas.neighbors_roots_count(&pixel) > 0 {
                            pix_count += 1;
                            if pixel.sign_change_on_lattice(f, &canvas, scan_lattice){
                                max_boost = cmp::max(iteration, max_boost);
                                canvas.set_pixel(&pixel, 0);
                                update_count += 1;
                                last_roots.push(pixel);
                            }
                        }
                    }
                }
            }
            println!("Scanned {} pixels for new roots, found: {} at lattice {}", pix_count, update_count, scan_lattice);

            while last_roots.len() !=0 {
                let mut pix_count = 0;
                let mut new_roots: Vec<Pixel> = Vec::new();
                for last_root in last_roots.iter(){
                    for neighbor in canvas.get_neighbors(&last_root){
                        if scandepth[neighbor.index] < deepscan_lattice as u16{
                            scandepth[neighbor.index] = deepscan_lattice as u16;
                            pix_count += 1;
                            if canvas.get_pixel(&neighbor) != 0 as PixelColor{
                                if neighbor.sign_change_on_lattice(f, &canvas, deepscan_lattice){
                                    max_boost = cmp::max(iteration, max_boost);
                                    canvas.set_pixel(&neighbor, 0);
                                    update_count += 1;
                                    new_roots.push(neighbor);
                                }
                            }
                        }
                    }
                }
                println!("Deep scan {} pixels (neighbors of {} new roots), found {} more new neighbor roots at lattice {}", pix_count, last_roots.len(), new_roots.len(), deepscan_lattice);
                last_roots = new_roots.clone();
            }
            println!("Updating: iteration {}, found {} additional pixels", iteration, update_count);
        }
        update_count=-1;
    }
    iteration
}

fn main() {
    // let picture = (
    //     |x:Float, y:Float| x.sin()-y,
    //     1.92*2.0,
    //     1.08*2.0,
    // );
    // let picture = (
    //     |x:Float ,y:Float| (x*x).sin() - (y*y).cos(),
    //     1.92*20.0,
    //     1.08*20.0,
    //     "circles"
    // );
    // let picture = (
    //     |x:Float, y:Float| sin(x)/sin(y)-cos(x*y),
    //     1.92*64.0,
    //     1.08*64.0,
    //     "wiggle-squares"
    // );
    // let picture = (|x:Float, y:Float| sin(1.0/x)-sin(1.0/y), 1.92*5.0, 1.08/5.0, "curve in cross");
    // let picture = (|x:Float, y:Float| sin(x)-cos(y)-sin(x/cos(y)), 1.92*100.0, 1.08*11.8, "beads");
    // let picture = (|x:Float, y:Float| sin(x*x/y)-cos(y*y/x), 1.92*100.0, 1.08*100.0, "butterfly");
    // let picture = (|x:Float, y:Float| x-y, 300.0, 3.0, "butterfly");
    // let picture = (|x:Float, y:Float| sin(x/y)-sin(y/x), 1.92*100.0, 1.08/100.0, "?");
    // let picture = (|x:Float, y:Float| (sin(x)+sin(y/2.0))*(sin(x)+sin(x/2.0)-y), 1.92*20.0, 1.08*20.0, "two quarters");
    // let picture = (|x:Float, y:Float| (x*x+y*y)*sin(x*y)-PI, 1.92*42.0, 1.08*42.0, "muare");
    // let picture = (|x:Float, y:Float| (x*x+y*y)*sin(x*y)-PI, 1.92*100.0, 1.08*100.0, "darkness come");
    // let picture = (|x:Float, y:Float| (x*x+y*y)*sin(x*y)-PI, 1.92*470.0, 1.08*470.0, "sea of solicitude");
    // let picture = (|x:Float, y:Float| sin(x*cos(y))-cos(y*sin(x)), 1.92*60.0, 1.08*60.0, "tarnished lace");
    let picture = (|x:Float, y:Float| sin(x/y)-cos(y/x)+x-y, 1.92*2.8, 1.08*2.8, "trimed knot");
    let mut canvas = Canvas::new(
        1920,1080,
        picture.1, picture.2,
        1920/2, 1080/2
    );
    let now = Instant::now();
    render(&mut canvas, &picture.0, 2);
    println!("Rendered in {:#?}, {} roots", now.elapsed(), canvas.roots());
    up_render(&mut canvas, &picture.0, 7);
    println!("Rendered and uprendered in {:#?}, {}", now.elapsed(), canvas.roots());
    clarify(&mut canvas, &picture.0, 11);
    println!("Finish rendering and updates in {:#?}, found {} roots", now.elapsed(), canvas.roots());
    show_and_wait(canvas);
}
