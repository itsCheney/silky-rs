//! `silky-tool`: inspect the files of a Silky's engine game.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use silky_format::akb;
use silky_format::arc::Archive;
use silky_format::game::{Game, Kind};
use silky_format::map::MapFile;
use silky_format::mes::{self, Operand, Profile, Script};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the archives of a game folder.
    Info { game: PathBuf },
    /// List the entries of one archive file.
    Ls { archive: PathBuf },
    /// Extract every entry of one archive file.
    Extract { archive: PathBuf, out: PathBuf },
    /// Decode an AKB image of a game to PNG.
    Akb {
        game: PathBuf,
        name: String,
        out: PathBuf,
    },
    /// Disassemble a script.
    Dis {
        game: PathBuf,
        name: String,
        /// Read PUSH operands as little endian (older builds).
        #[arg(long)]
        push_le: bool,
    },
    /// Print one section of a MAP text table.
    Map {
        game: PathBuf,
        name: String,
        #[arg(long, default_value_t = 0)]
        section: usize,
    },
    /// Decode every script, MAP table and image header of a game, and check
    /// that jumps and offset tables land on instructions.
    Check {
        game: PathBuf,
        #[arg(long)]
        push_le: bool,
        /// Also decode every image fully (slow).
        #[arg(long)]
        images: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Info { game } => info(&game),
        Command::Ls { archive } => {
            let a = Archive::open(&archive)?;
            for e in a.entries() {
                println!("{:>10} {:>10} {}", e.packed_size, e.size, e.name);
            }
            Ok(())
        }
        Command::Extract { archive, out } => {
            let a = Archive::open(&archive)?;
            fs::create_dir_all(&out)?;
            for e in a.entries() {
                fs::write(out.join(&e.name), a.read(e)?)?;
            }
            println!("{} files", a.entries().len());
            Ok(())
        }
        Command::Akb { game, name, out } => {
            let game = Game::open(&game)?;
            let img = load_image(&game, &name, &mut HashMap::new())?;
            write_png(&out, &img)
        }
        Command::Dis {
            game,
            name,
            push_le,
        } => dis(
            &Game::open(&game)?,
            &name,
            Profile {
                push_big_endian: !push_le,
            },
        ),
        Command::Map {
            game,
            name,
            section,
        } => {
            let map = MapFile::parse(&Game::open(&game)?.set(Kind::Data).read(&name)?)?;
            for (number, text) in map.entries(section) {
                println!("{number:>6} {text}");
            }
            Ok(())
        }
        Command::Check {
            game,
            push_le,
            images,
        } => check(
            &Game::open(&game)?,
            Profile {
                push_big_endian: !push_le,
            },
            images,
        ),
    }
}

fn info(path: &Path) -> Result<()> {
    let game = Game::open(path)?;
    for kind in Kind::ALL {
        let set = game.set(kind);
        let files: Vec<String> = set
            .archives()
            .iter()
            .map(|a| {
                format!(
                    "{} ({})",
                    a.path().file_name().unwrap().to_string_lossy(),
                    a.entries().len()
                )
            })
            .collect();
        println!(
            "{:<7} {:>6} files  {}",
            kind.name(),
            set.names().len(),
            files.join(", ")
        );
    }
    Ok(())
}

/// Loads an image, resolving `AKB+` base images.
fn load_image(
    game: &Game,
    name: &str,
    cache: &mut HashMap<String, akb::Image>,
) -> Result<akb::Image> {
    let key = name.to_ascii_lowercase();
    if let Some(img) = cache.get(&key) {
        return Ok(img.clone());
    }
    let data = game
        .set(Kind::Layer)
        .read(name)
        .with_context(|| format!("reading {name}"))?;
    let header = akb::Header::parse(&data)?;
    let base = match &header.base {
        Some(b) if b.eq_ignore_ascii_case(name) => bail!("{name} names itself as base"),
        Some(b) => Some(load_image(game, &with_ext(b), cache)?),
        None => None,
    };
    let img = akb::decode(&data, base.as_ref())?;
    cache.insert(key, img.clone());
    Ok(img)
}

/// Base names are stored without or with an extension.
fn with_ext(name: &str) -> String {
    if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.akb")
    }
}

fn write_png(path: &Path, img: &akb::Image) -> Result<()> {
    let file = BufWriter::new(fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&img.rgba)?;
    Ok(())
}

fn map_for(game: &Game, script: &str) -> Option<MapFile> {
    let stem = script.rsplit_once('.').map_or(script, |(s, _)| s);
    let data = game.set(Kind::Data).read(&format!("{stem}.map")).ok()?;
    MapFile::parse(&data).ok()
}

fn dis(game: &Game, name: &str, profile: Profile) -> Result<()> {
    let script = Script::parse(&game.set(Kind::Script).read(name)?)?;
    let labels: HashMap<u32, usize> = script
        .table_a
        .iter()
        .enumerate()
        .map(|(i, &o)| (o, i))
        .collect();
    println!(
        "; {name}: table_a {} entries, table_b {} entries",
        script.table_a.len(),
        script.table_b.len()
    );
    for ins in script.disassemble(profile)? {
        if let Some(i) = labels.get(&ins.offset) {
            println!("a{i}:");
        }
        let operand = match &ins.operand {
            Operand::None => String::new(),
            Operand::U8(v) => v.to_string(),
            Operand::Int(v) => v.to_string(),
            Operand::Target(t) => format!("@{t:06x}"),
            Operand::Str(_) => format!("{:?}", ins.text().unwrap()),
            Operand::Message(n) => n.to_string(),
        };
        println!(
            "{:06x}  {:<11} {operand}",
            ins.offset,
            mes::mnemonic(ins.opcode)
        );
    }
    Ok(())
}

fn check(game: &Game, profile: Profile, images: bool) -> Result<()> {
    let mut problems = 0usize;
    let mut opcodes: BTreeMap<u8, usize> = BTreeMap::new();
    let mut calls: BTreeMap<String, usize> = BTreeMap::new();
    let scripts = game.set(Kind::Script);
    let names: Vec<String> = scripts
        .names()
        .into_iter()
        .filter(|n| {
            let l = n.to_ascii_lowercase();
            l.ends_with(".mes") || l.ends_with(".lib")
        })
        .collect();
    let mut instr_total = 0;
    let mut messages_total = 0;
    let mut map_texts = 0;
    for name in &names {
        let script = match Script::parse(&scripts.read(name)?) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: {e}");
                problems += 1;
                continue;
            }
        };
        // Decode linearly; on an unknown opcode report and skip one byte.
        let mut starts = std::collections::HashSet::new();
        let mut instrs = Vec::new();
        let mut pos = 0u32;
        while (pos as usize) < script.code.len() {
            match script.decode_at(pos, profile) {
                Ok((ins, next)) => {
                    starts.insert(ins.offset);
                    instrs.push(ins);
                    pos = next;
                }
                Err(e) => {
                    println!("{name}: {e}");
                    problems += 1;
                    pos += 1;
                }
            }
        }
        instr_total += instrs.len();
        let script_messages = instrs
            .iter()
            .filter(|i| matches!(i.operand, Operand::Message(_)))
            .count();
        let map_entries = map_for(game, name).map(|m| m.len(0));
        if script_messages > 0 && map_entries.is_none() {
            // Scene replay scripts reuse the story scripts' texts.
            println!("note: {name}: {script_messages} messages, no MAP table of its own");
        }
        map_texts += map_entries.unwrap_or(0);
        for (i, ins) in instrs.iter().enumerate() {
            *opcodes.entry(ins.opcode).or_default() += 1;
            match &ins.operand {
                Operand::Target(t) if !starts.contains(t) => {
                    println!(
                        "{name}: {:#x} jumps into an instruction ({t:#x})",
                        ins.offset
                    );
                    problems += 1;
                }
                Operand::Message(_) => messages_total += 1,
                _ => {}
            }
            // A call pushes "<function>", "<module>.inc" as two strings.
            if let (Some(f), Some(next)) = (ins.text(), instrs.get(i + 1).and_then(|n| n.text())) {
                if next.ends_with(".inc") && ins.opcode == 0x33 {
                    *calls.entry(format!("{next}:{f}")).or_default() += 1;
                }
            }
        }
        for (table, offsets) in [("a", &script.table_a), ("b", &script.table_b)] {
            for (i, o) in offsets.iter().enumerate() {
                if !starts.contains(o) {
                    println!("{name}: table_{table}[{i}] = {o:#x} is not an instruction");
                    problems += 1;
                }
            }
        }
    }
    println!("scripts: {} files, {instr_total} instructions", names.len());
    println!("messages: {messages_total} in scripts, {map_texts} texts in their MAP tables");
    println!(
        "opcodes: {}",
        opcodes
            .iter()
            .map(|(o, n)| format!("{o:02x}:{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("called functions ({}):", calls.len());
    for (f, n) in &calls {
        println!("  {f} x{n}");
    }

    let data = game.set(Kind::Data);
    let maps = data
        .names()
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".map"))
        .collect::<Vec<_>>();
    for name in &maps {
        if let Err(e) = MapFile::parse(&data.read(name)?) {
            println!("{name}: {e}");
            problems += 1;
        }
    }
    println!("MAP tables: {}", maps.len());

    let layer = game.set(Kind::Layer);
    let akbs = layer.names();
    let mut cache = HashMap::new();
    for name in &akbs {
        let result = if images {
            load_image(game, name, &mut cache).map(|_| ())
        } else {
            layer
                .read(name)
                .map_err(Into::into)
                .and_then(|d| akb::Header::parse(&d).map(|_| ()).map_err(Into::into))
        };
        if let Err(e) = result {
            println!("{name}: {e:#}");
            problems += 1;
        }
        if cache.len() > 64 {
            cache.clear();
        }
    }
    println!(
        "images: {} ({})",
        akbs.len(),
        if images { "decoded" } else { "headers" }
    );
    if problems > 0 {
        bail!("{problems} problems");
    }
    println!("OK");
    Ok(())
}
