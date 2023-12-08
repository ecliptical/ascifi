use gumdrop::{parse_args_default_or_exit, Options};
use image::{imageops::FilterType, open, GrayImage};
use std::{
    collections::HashMap,
    io::{stdin, stdout, BufRead as _, Write},
    iter,
    path::PathBuf,
};

const CODES: &str =
    " `1234567890-=~!@#$%^&*()_+qwertyuiop[]QWERTYUIOP{}|asdfghjkl;'ASDFGHJKL:zxcvbnm,./ZXCVBNM<>?";

#[derive(Debug, Options)]
struct Args {
    /// Path to image file.
    #[options(free)]
    file: Option<PathBuf>,

    /// Color palette, in the order of intensity.
    #[options(no_short, default = " ░▒▓█")]
    colors: String,

    /// Desired image width in characters.
    #[options(default = "160")]
    width: u32,

    /// Squash image vertically to 1/2 its original height.
    squash: bool,

    /// Generate compressed text.
    compress: bool,

    /// Display help.
    help: bool,
}

fn generate<T: Write>(out: &mut T, image: GrayImage, colors: &str) -> anyhow::Result<()> {
    let chars: Vec<_> = colors.chars().collect();
    let divider = (u8::MAX as f64 + 1f64) / (chars.len() as f64);

    for row in image.rows() {
        for pixel in row {
            let index = pixel.0[0] as f64 / divider;
            let color = chars[index as usize];
            write!(out, "{}", color)?;
        }

        writeln!(out)?;
    }

    Ok(())
}

fn compress(buf: &[u8], colors: &str) -> anyhow::Result<()> {
    let colors: HashMap<_, _> = colors.chars().enumerate().map(|(i, c)| (c, i)).collect();
    let index_chars: Vec<_> = CODES.chars().take(colors.len()).collect();
    let value_chars: Vec<_> = CODES.chars().skip(colors.len()).collect();
    let mut image = String::default();
    for line in buf.lines() {
        image.push_str(&line?);
    }

    let chars: Vec<_> = image.chars().collect();

    let mut start = 0;
    while start < chars.len() {
        let mut end = start + 1;
        while end < chars.len() && chars[end] == chars[start] && end - start <= value_chars.len() {
            end += 1;
        }

        let color = colors.get(&chars[start]).copied().unwrap_or_default();
        print!("{}", index_chars[color]);
        if end > start + 1 {
            print!("{}", value_chars[end - start - 2]);
        }

        start = end;
    }

    println!();
    Ok(())
}

fn decompress(data: &str, width: u32, colors: &str) -> anyhow::Result<()> {
    let chars: Vec<_> = colors.chars().collect();
    let indices: HashMap<_, _> = CODES.chars().take(chars.len()).enumerate().map(|(i, c)| (c, i)).collect();
    let values: HashMap<_, _> = CODES.chars().skip(chars.len()).enumerate().map(|(i, c)| (c, i + 1)).collect();

    let mut image: Vec<_> = Vec::default();

    let mut data = data.chars().peekable();
    while let Some(c) = data.next() {
        let index = indices.get(&c).copied().unwrap_or_default();
        let color = chars[index];
        image.push(color);
        if let Some(repeat) = data.next_if(|c| values.contains_key(c)).and_then(|c| values.get(&c)).copied() {
            image.extend(iter::repeat(color).take(repeat));
        }
    }

    for line in image.chunks(width as usize) {
        let line: String = line.iter().collect();
        println!("{}", line);
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Args = parse_args_default_or_exit();

    if let Some(file)= args.file {
        let image = open(file)?;
        let ratio = image.width() as f64 / (args.width as f64);
        let mut height = image.height() as f64 / ratio;
        if args.squash {
            height /= 2f64;
        }

        let image = image.resize_exact(args.width, height as u32, FilterType::Lanczos3);
        let image = image.grayscale();
        let image = image.into_luma8();

        if args.compress {
            let mut buf = Vec::with_capacity(8192);
            generate(&mut buf, image, &args.colors)?;
            compress(&buf, &args.colors)?;
        } else {
            let mut out = stdout().lock();
            generate(&mut out, image, &args.colors)?;
        }
    } else {
        if let Some(Ok(line)) = stdin().lines().next() {
            decompress(&line, args.width, &args.colors)?;
        }
    }

    Ok(())
}
