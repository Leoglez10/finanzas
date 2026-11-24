//! Bank statement PDF → rows of cells. Glyph positions are grouped into lines and cells, and cells are
//! placed under the header's columns, so a charge and a deposit stay in different columns. The rows then
//! go through the same column detection as CSV and Excel files.
use pdf_extract::{MediaBox, OutputDev, OutputError, Transform};

struct Glyph { page: u32, x: f64, end: f64, y: f64, size: f64, text: String }

#[derive(Default)]
struct Collect { page: u32, flip: Option<Transform>, glyphs: Vec<Glyph> }

impl OutputDev for Collect {
    fn begin_page(&mut self, page: u32, media: &MediaBox, _: Option<(f64, f64, f64, f64)>) -> Result<(), OutputError> {
        self.page = page;
        // PDF y grows upwards; flipped so lines sort top to bottom.
        self.flip = Some(Transform::row_major(1., 0., 0., -1., 0., media.ury - media.lly));
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn output_character(&mut self, trm: &Transform, width: f64, _: f64, font_size: f64, text: &str) -> Result<(), OutputError> {
        let at = trm.post_transform(&self.flip.unwrap_or_else(Transform::identity));
        let size = font_size * (trm.m11 * trm.m22 - trm.m12 * trm.m21).abs().sqrt();
        self.glyphs.push(Glyph { page: self.page, x: at.m31, end: at.m31 + width * size, y: at.m32, size, text: text.into() });
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn end_word(&mut self) -> Result<(), OutputError> { Ok(()) }
    fn end_line(&mut self) -> Result<(), OutputError> { Ok(()) }
}

struct Cell { x: f64, end: f64, text: String }

const HEADER_WORDS: [&str; 12] = ["fecha", "date", "descrip", "concepto", "cargo", "abono", "importe", "monto", "saldo", "amount", "retiro", "deposito"];

pub fn rows(path: &std::path::Path) -> Result<Vec<Vec<String>>, String> {
    let mut doc = pdf_extract::Document::load(path).map_err(|e| format!("No pude leer el PDF: {e}"))?;
    if doc.is_encrypted() && doc.decrypt("").is_err() {
        return Err("El PDF tiene contraseña. Ábrelo, guárdalo sin contraseña (o expórtalo a Excel) e inténtalo de nuevo".into());
    }
    let mut collect = Collect::default();
    pdf_extract::output_doc(&doc, &mut collect).map_err(|e| format!("No pude leer el PDF: {e}"))?;
    let rows = table(lines(collect.glyphs));
    if rows.is_empty() { return Err("El PDF no tiene texto (quizá es una imagen escaneada)".into()); }
    Ok(rows)
}

/// Glyphs on the same baseline form a line; a gap wider than most of a character starts a new cell.
fn lines(mut glyphs: Vec<Glyph>) -> Vec<Vec<Cell>> {
    glyphs.sort_by(|a, b| (a.page, a.y).partial_cmp(&(b.page, b.y)).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<Vec<Glyph>> = vec![];
    for g in glyphs {
        match out.last_mut() {
            Some(line) if line[0].page == g.page && (line[0].y - g.y).abs() < g.size * 0.5 => line.push(g),
            _ => out.push(vec![g]),
        }
    }
    out.into_iter().map(|mut line| {
        line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let mut cells: Vec<Cell> = vec![];
        for g in line {
            match cells.last_mut() {
                Some(c) if g.x - c.end < g.size * 0.8 => {
                    if g.x - c.end > g.size * 0.15 && !c.text.ends_with(' ') { c.text.push(' '); }
                    c.text.push_str(&g.text);
                    c.end = c.end.max(g.end);
                }
                _ => cells.push(Cell { x: g.x, end: g.end, text: g.text }),
            }
        }
        cells.into_iter().map(|c| Cell { text: c.text.trim().to_string(), ..c }).filter(|c| !c.text.is_empty()).collect::<Vec<_>>()
    }).filter(|l: &Vec<Cell>| !l.is_empty()).collect()
}

fn is_header(line: &[Cell]) -> bool {
    line.len() >= 3 && line.iter().filter(|c| { let t = c.text.to_lowercase(); HEADER_WORDS.iter().any(|w| t.contains(w)) }).count() >= 2
}

/// Lines after the header get one cell per header column: the column the cell overlaps most, or the
/// nearest one. Repeated headers (one per page) are dropped; lines before the header stay as they are.
fn table(lines: Vec<Vec<Cell>>) -> Vec<Vec<String>> {
    let Some(h) = lines.iter().position(|l| is_header(l)) else {
        return lines.into_iter().map(|l| l.into_iter().map(|c| c.text).collect()).collect();
    };
    let names: Vec<String> = lines[h].iter().map(|c| c.text.clone()).collect();
    let columns: Vec<(f64, f64)> = lines[h].iter().map(|c| (c.x, c.end)).collect();
    let mut out: Vec<Vec<String>> = lines[..h].iter().map(|l| l.iter().map(|c| c.text.clone()).collect()).collect();
    out.push(names.clone());
    for line in &lines[h + 1..] {
        if line.iter().map(|c| c.text.as_str()).eq(names.iter().map(String::as_str)) { continue; }
        let mut row = vec![String::new(); columns.len()];
        for cell in line {
            let overlap = |&(a, b): &(f64, f64)| cell.end.min(b) - cell.x.max(a);
            let distance = |&(a, b): &(f64, f64)| ((a + b) / 2. - (cell.x + cell.end) / 2.).abs();
            let best = (0..columns.len()).max_by(|&i, &j| {
                let (oi, oj) = (overlap(&columns[i]), overlap(&columns[j]));
                if oi > 0. || oj > 0. { oi.total_cmp(&oj) } else { distance(&columns[j]).total_cmp(&distance(&columns[i])) }
            }).unwrap_or(0);
            if !row[best].is_empty() { row[best].push(' '); }
            row[best].push_str(&cell.text);
        }
        out.push(row);
    }
    out
}
