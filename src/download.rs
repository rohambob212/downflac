use indicatif::{ProgressBar, ProgressStyle};
use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    picture::{Picture, PictureType},
    prelude::*,
    tag::Tag,
};
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn watch_pause(bar: ProgressBar) -> Option<Arc<AtomicBool>> {
    use crossterm::event::{Event, KeyCode, KeyEventKind, poll, read};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    enable_raw_mode().ok()?;
    let paused = Arc::new(AtomicBool::new(false));
    let flag = paused.clone();
    thread::spawn(move || {
        loop {
            if !matches!(poll(Duration::from_millis(100)), Ok(true)) {
                continue;
            }
            match read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let now = !flag.fetch_xor(true, Ordering::SeqCst);
                        let msg = if now {
                            "-- paused, Enter or space to resume --"
                        } else {
                            "-- resuming --"
                        };
                        bar.suspend(|| println!("{msg}"));
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        let _ = disable_raw_mode();
                        return;
                    }
                    _ => {}
                },
                Err(_) => return,
                _ => {}
            }
        }
    });
    Some(paused)
}

fn pump(reader: &mut impl Read, out: &mut impl Write, paused: &AtomicBool) -> io::Result<()> {
    let mut buf = [0u8; 64 * 1024];
    loop {
        while paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(20));
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        out.write_all(&buf[..n])?;
    }
}

pub fn download(link: String, pth: String) -> Result<()> {
    let mut downer = ureq::get(link).call()?.into_body();
    let progress = ProgressBar::new(downer.content_length().unwrap_or(0));
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )?
        .progress_chars("#>-"),
    );
    let mut reader = progress.wrap_read(downer.as_reader());
    let mut out = File::create(pth)?;

    println!("downloading... Enter or space to pause, Esc to hide this hint");
    let watched = watch_pause(progress.clone());
    let paused = watched.clone().unwrap_or_default();
    let result = pump(&mut reader, &mut out, &paused);
    if watched.is_some() {
        let _ = crossterm::terminal::disable_raw_mode();
    }
    result?;
    progress.finish();
    Ok(())
}

pub fn embed_cover(path: String, cover_url: String) -> Result<()> {
    let data = ureq::get(cover_url).call()?.body_mut().read_to_vec()?;
    let mut pic = Picture::from_reader(&mut data.as_slice())?;
    pic.set_pic_type(PictureType::CoverFront);

    let mut file = lofty::read_from_path(&path)?;
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(file.primary_tag_type()));
    }
    let tag = file.primary_tag_mut().unwrap();
    tag.push_picture(pic);
    tag.save_to_path(path, WriteOptions::default())?;
    Ok(())
}
