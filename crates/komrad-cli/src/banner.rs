use figlet_rs::FIGfont;
use owo_colors::OwoColorize;
use palette::{LinSrgb, Mix};
use tracing::warn;

const BANNER_TEXT: &str = "komrad";
// const FONT_NAME: &str = "ANSI Shadow";

pub fn banner() {
    let stops = vec![
        (0.0, LinSrgb::new(0.8, 0.0, 0.4)),
        (0.3, LinSrgb::new(0.75, 0.4, 0.8)),
        (0.7, LinSrgb::new(0.7, 0.6, 0.0)),
        (1.0, LinSrgb::new(0.3, 0.7, 0.9)),
    ];

    let banner = gradient_banner(BANNER_TEXT, &stops);
    warn!("\n{}", banner);
}

pub fn gradient_banner(text: &str, stops: &[(f32, LinSrgb)]) -> String {
    let standard_font =
        FIGfont::from_content(include_str!("../../../assets/fonts/ANSI Shadow.flf")).unwrap();
    // let standard_font =
    //     FIGfont::from_file(format!("assets/fonts/{}.flf", FONT_NAME).as_str()).unwrap();
    let figure = standard_font.convert(text).unwrap();
    let ascii_text = figure.to_string();

    let lines: Vec<&str> = ascii_text.lines().collect();
    let lines = trim_blank_lines_around(&lines);
    let num_lines = lines.len();

    let mut output = String::new();

    for (y, line) in lines.iter().enumerate() {
        let t = y as f32 / (num_lines.max(1) as f32);
        let color = interpolate_color(t, stops);

        let (r, g, b) = (
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8,
        );

        output.push_str(&line.truecolor(r, g, b).to_string());
        output.push('\n');
    }

    output
}

fn trim_blank_lines_around<'a>(lines: &'a [&'a str]) -> &'a [&'a str] {
    let start = lines
        .iter()
        .position(|&line| !line.trim().is_empty())
        .unwrap_or(0);
    let end = lines
        .iter()
        .rposition(|&line| !line.trim().is_empty())
        .unwrap_or(lines.len() - 1);
    &lines[start..=end]
}

fn interpolate_color(t: f32, stops: &[(f32, LinSrgb)]) -> LinSrgb {
    for w in stops.windows(2) {
        let (t1, c1) = w[0];
        let (t2, c2) = w[1];

        if t >= t1 && t <= t2 {
            let local_t = (t - t1) / (t2 - t1);
            return c1.mix(c2, local_t);
        }
    }
    stops.last().unwrap().1 // Fallback to last color if out of bounds
}
