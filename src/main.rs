use figlet_rs::FIGfont;

fn main() {
    let standard_font = FIGfont::standard().unwrap();
    //let my_font = //
    let figure = standard_font.convert("KoMRaD");
    assert!(figure.is_some());
    println!("{}", figure.unwrap());
}

