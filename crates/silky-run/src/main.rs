//! Headless runner. For now it runs the scripts with every engine function
//! logged instead of performed, which shows the order of engine calls and
//! the story text without any graphics or audio.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use silky_format::game::{Game, Kind};
use silky_format::map::MapFile;
use silky_format::mes::{Profile, Script};
use silky_vm::native::{self, script, GAME, SCRIPT};
use silky_vm::{Flow, Host, Stop, Value, Vm};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the game's scripts headless and print every engine call.
    Trace {
        /// Game folder (with Patch.xml or the archives).
        game: PathBuf,
        /// Script to start with.
        #[arg(long, default_value = "start.mes")]
        start: String,
        /// Text language: section of the MAP tables (0 = first).
        #[arg(long, default_value_t = 0)]
        lang: usize,
        /// Option to take at every selection (from 0).
        #[arg(long, default_value_t = 0)]
        choose: usize,
        /// Stop after this many engine calls.
        #[arg(long, default_value_t = 2000)]
        max_calls: u64,
        /// Print only story text and selections.
        #[arg(long)]
        text_only: bool,
        /// Little-endian PUSH operands (older builds).
        #[arg(long)]
        push_le: bool,
        /// Result of an engine call, as CLASS.SUB=VALUE (for class 29 the
        /// command number), e.g. 29.110=1 picks "Start" on the title menu.
        /// Other calls that return a value return 0.
        #[arg(long = "result", value_parser = parse_result)]
        results: Vec<((i32, i32), i32)>,
    },
    /// Run the game headless with the runtime and save frames as PNG: the
    /// title screen and the chosen message lines.
    Snap {
        /// Game folder (with Patch.xml or the archives).
        game: PathBuf,
        /// Folder for the PNG files.
        #[arg(long, default_value = "snaps")]
        out: PathBuf,
        /// Message lines to save, counted from the start of the game
        /// (0 = first line), comma separated.
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "0,1,2,3,4,5,10,20,50,100"
        )]
        lines: Vec<u64>,
        /// Text language: 0 English, 1 Simplified Chinese, 2 Traditional Chinese.
        #[arg(long, default_value_t = 0)]
        lang: usize,
        /// Font files for the message text, in fallback order. Without
        /// one, the text is left out of the frames.
        #[arg(long)]
        font: Vec<PathBuf>,
        /// Option to take at every selection.
        #[arg(long, default_value_t = 0)]
        choose: usize,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Trace {
            game,
            start,
            lang,
            choose,
            max_calls,
            text_only,
            push_le,
            results,
        } => {
            let game = Game::open(&game)
                .with_context(|| format!("cannot open game folder {}", game.display()))?;
            let profile = Profile {
                push_big_endian: !push_le,
            };
            let options = TraceOptions {
                lang,
                choose,
                max_calls,
                text_only,
                results: results.into_iter().collect(),
            };
            trace(game, profile, &start, options)
        }
        Command::Snap {
            game,
            out,
            lines,
            lang,
            font,
            choose,
        } => {
            let game = Game::open(&game)
                .with_context(|| format!("cannot open game folder {}", game.display()))?;
            snap(game, &out, &lines, lang, &font, choose)
        }
    }
}

fn snap(
    game: Game,
    out: &std::path::Path,
    lines: &[u64],
    lang: usize,
    fonts: &[PathBuf],
    choose: usize,
) -> Result<()> {
    use silky_runtime::render::Renderer;
    use silky_runtime::text::TextRenderer;
    use silky_runtime::ui::{Layout, Ui};
    use silky_runtime::{Assets, Config, Language, Pause, Runtime};

    std::fs::create_dir_all(out)?;
    let language = Language::from_section(lang).context("--lang must be 0, 1 or 2")?;
    let text = if fonts.is_empty() {
        println!("no --font given: frames are saved without message text");
        None
    } else {
        let files = fonts
            .iter()
            .map(|f| std::fs::read(f).with_context(|| format!("reading {}", f.display())))
            .collect::<Result<Vec<_>>>()?;
        Some(TextRenderer::new(&files)?)
    };
    let mut renderer = Renderer::new(text, language);
    let config = Config {
        language,
        ..Config::default()
    };
    let assets = Assets::new(game);
    let layout = Layout::load(&assets, language)?;
    let mut rt = Runtime::new(assets, config);
    let mut vm = Vm::new(Profile::default());
    rt.boot(&mut vm, "start.mes")?;
    let last = lines.iter().copied().max().unwrap_or(0);
    let mut line_no = 0u64;
    let mut title_saved = false;
    loop {
        match vm.run(&mut rt, 50_000_000)? {
            Stop::Yield => match rt.pause.take() {
                Some(Pause::Title) => {
                    if !title_saved {
                        let mut frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let ui = Ui {
                            title: Some(false),
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                        let path = out.join("title.png");
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                        title_saved = true;
                    }
                    let choice = rt.config.title_choice;
                    rt.answer_title(&mut vm, choice);
                }
                Some(Pause::Text { index }) => {
                    if lines.contains(&line_no) {
                        let mut frame = renderer.render(&rt.scene, &mut rt.assets)?;
                        let ui = Ui {
                            waiting: true,
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, true, None)?;
                        let script = vm.script_name().unwrap_or("x").replace('.', "_");
                        let path = out.join(format!("line{line_no:05}-{script}-{index}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    if line_no >= last {
                        break;
                    }
                    line_no += 1;
                }
                Some(Pause::Choice) => {
                    println!("select [{}] -> {}", rt.choices.join(" | "), choose);
                    let pick = choose.min(rt.choices.len().saturating_sub(1));
                    rt.answer_choice(&mut vm, pick);
                }
                _ => {}
            },
            Stop::Options => {
                rt.begin_options(vm.options().len());
                vm.run_all_options()?;
            }
            Stop::Ended => break,
            Stop::Budget => anyhow::bail!("no message line within the step budget"),
        }
    }
    println!(
        "{} engine calls carried out; not yet carried out:",
        rt.handled
    );
    let mut skipped: Vec<_> = rt.unhandled.iter().collect();
    skipped.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in skipped {
        println!("  {name:<16} {count}");
    }
    Ok(())
}

fn write_png(path: &std::path::Path, frame: &silky_runtime::render::Canvas) -> Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, frame.width, frame.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&frame.rgba)?;
    Ok(())
}

fn load_script(game: &Game, name: &str) -> Result<Script> {
    let data = game
        .set(Kind::Script)
        .read(name)
        .with_context(|| format!("cannot read script {name}"))?;
    Ok(Script::parse(&data)?)
}

fn parse_result(s: &str) -> std::result::Result<((i32, i32), i32), String> {
    let err = || format!("expected CLASS.SUB=VALUE, got {s:?}");
    let (key, value) = s.split_once('=').ok_or_else(err)?;
    let (class, sub) = key.split_once('.').ok_or_else(err)?;
    let num = |t: &str| t.trim().parse::<i32>().map_err(|_| err());
    Ok(((num(class)?, num(sub)?), num(value)?))
}

struct TraceOptions {
    lang: usize,
    choose: usize,
    max_calls: u64,
    text_only: bool,
    results: HashMap<(i32, i32), i32>,
}

fn trace(game: Game, profile: Profile, start: &str, options: TraceOptions) -> Result<()> {
    let max_calls = options.max_calls;
    let mut vm = Vm::new(profile);
    vm.jump_script(start, load_script(&game, start)?);
    let mut host = TraceHost {
        game,
        map: None,
        calls: 0,
        options,
    };
    loop {
        let stop = vm.run(&mut host, 1_000_000)?;
        match stop {
            Stop::Yield => {}
            Stop::Options => {
                // The option bodies register labels; the selection itself
                // is Game.20, answered with --choose.
                println!("options: {}", vm.options().len());
                vm.run_all_options()?;
            }
            Stop::Ended => {
                println!("end of script after {} instructions", vm.steps);
                break;
            }
            Stop::Budget => {}
        }
        if host.calls >= max_calls {
            println!(
                "stopped after {} engine calls ({} instructions)",
                host.calls, vm.steps
            );
            break;
        }
    }
    Ok(())
}

struct TraceHost {
    game: Game,
    /// Text table loaded by the script (name, table).
    map: Option<(String, MapFile)>,
    calls: u64,
    options: TraceOptions,
}

impl TraceHost {
    fn log(&self, vm: &Vm, what: &str) {
        if !self.options.text_only {
            println!(
                "{:>12} {:06x}  {what}",
                vm.script_name().unwrap_or("?"),
                vm.pc()
            );
        }
    }

    fn result(&self, vm: &mut Vm, class: i32, sub: i32) {
        if let Some(&v) = self.options.results.get(&(class, sub)) {
            vm.push(v);
        } else if (class, sub) == (GAME, 20) {
            println!("select -> option {}", self.options.choose);
            vm.push(self.options.choose as i32 + 1);
        } else if native::returns_value(class, sub) {
            vm.push(0);
        }
    }

    fn host_err(e: anyhow::Error) -> silky_vm::Error {
        silky_vm::Error::Host(format!("{e:#}"))
    }
}

fn join(args: &[Value]) -> String {
    args.iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

impl Host for TraceHost {
    fn native(&mut self, vm: &mut Vm, class: i32) -> silky_vm::Result<Flow> {
        self.calls += 1;
        let name = native::class_name(class).unwrap_or("?");
        match class {
            SCRIPT => {
                let sub = vm.pop_int()?;
                let file = vm.pop_str()?;
                self.log(vm, &format!("Script.{sub}({file:?})"));
                let s = load_script(&self.game, &file).map_err(Self::host_err)?;
                match sub {
                    script::JUMP => vm.jump_script(&file, s),
                    script::CALL | script::LIBRARY => vm.call_script(&file, s),
                    _ => return Err(silky_vm::Error::Host(format!("Script.{sub}"))),
                }
                return Ok(Flow::Continue);
            }
            GAME => {
                let mut args = vm.pop_statement();
                let Some(Value::Int(cmd)) = args.pop() else {
                    return Err(silky_vm::Error::Host("Game call without a command".into()));
                };
                // Command 23 shows story text from the loaded MAP table:
                // sub 2 loads a table, sub 0 shows the line at an index.
                if cmd == 23 {
                    match args.as_slice() {
                        [Value::Str(file), Value::Int(2)] => {
                            let data = self
                                .game
                                .set(Kind::Data)
                                .read(file)
                                .map_err(|e| silky_vm::Error::Host(e.to_string()))?;
                            let map = MapFile::parse(&data)?;
                            self.log(vm, &format!("Game.23.2 text table {file:?}"));
                            self.map = Some((file.clone(), map));
                            vm.push(0);
                        }
                        [Value::Int(index), Value::Int(0)] => {
                            let text = self
                                .map
                                .as_ref()
                                .and_then(|(_, m)| m.line(self.options.lang, *index as usize))
                                .unwrap_or("<no text>");
                            println!("{:>12} {:>6}  {text}", "text", index);
                        }
                        _ => self.log(vm, &format!("Game.23({})", join(&args))),
                    }
                    return Ok(Flow::Continue);
                }
                self.log(vm, &format!("Game.{cmd}({})", join(&args)));
                self.result(vm, class, cmd);
            }
            _ => {
                let mut args = vm.pop_statement();
                let sub = match args.pop() {
                    Some(Value::Int(s)) => s,
                    _ => -1,
                };
                self.log(vm, &format!("{name}.{sub}({})", join(&args)));
                self.result(vm, class, sub);
            }
        }
        Ok(Flow::Continue)
    }

    fn message(&mut self, vm: &mut Vm, number: u32) -> silky_vm::Result<Flow> {
        self.log(vm, &format!("message {number}"));
        Ok(Flow::Continue)
    }

    fn select_message(&mut self, vm: &mut Vm, number: u32) -> silky_vm::Result<Flow> {
        self.log(vm, &format!("select message {number}"));
        Ok(Flow::Continue)
    }
}
