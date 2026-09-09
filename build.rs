use image::{
    ExtendedColorType,
    codecs::ico::{IcoEncoder, IcoFrame},
    imageops::FilterType,
};
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=Logo.png");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let original = image::open("Logo.png").expect("read the supplied logo");
    // Extract the calculator mark for small native icon surfaces. The supplied
    // 1280px artwork and its NumNote wordmark remain untouched in Logo.png.
    let mark = original.crop_imm(360, 276, 540, 540);
    mark.resize_exact(256, 256, FilterType::Lanczos3)
        .save(out.join("app-icon.png"))
        .expect("generate app icon");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let frames: Vec<_> = [16, 24, 32, 48, 64, 128, 256]
            .into_iter()
            .map(|size| {
                let rgba = mark
                    .resize_exact(size, size, FilterType::Lanczos3)
                    .into_rgba8();
                IcoFrame::as_png(&rgba, size, size, ExtendedColorType::Rgba8).unwrap()
            })
            .collect();
        let icon = out.join("app.ico");
        IcoEncoder::new(fs::File::create(&icon).unwrap())
            .encode_images(&frames)
            .unwrap();
        let rc = out.join("app.rc");
        fs::write(
            &rc,
            format!(
                "1 ICON \"{}\"\n",
                icon.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();
        embed_resource::compile(&rc, embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
