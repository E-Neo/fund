use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

const CHINESE_FONT_NAME: &str = "NotoSansMonoCJKsc-Regular";

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let chinese_font_path = Path::new(&out_dir).join("chinese_font");
    let font = font_kit::source::SystemSource::new()
        .select_by_postscript_name(CHINESE_FONT_NAME)?
        .load()?;
    let mut f = BufWriter::new(File::create(&chinese_font_path)?);
    f.write_all(font.copy_font_data().unwrap().as_slice())?;
    Ok(())
}
