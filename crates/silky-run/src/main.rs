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

/// Native class of the backlog.
const BACKLOG: i32 = 22;
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
        /// Text language: en, zh-hans, zh-hant or ja (0, 1, 2 also name the
        /// sections of the MAP tables). Default: the game's first language.
        #[arg(long)]
        lang: Option<String>,
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
        /// Several values, as 29.42=1,1,2, are returned by successive calls,
        /// the last one from then on. Other calls that return a value
        /// return 0.
        #[arg(long = "result", value_parser = parse_result)]
        results: Vec<((i32, i32), Vec<i32>)>,
        /// Also write the calls to this trace file (see docs/trace.md).
        #[arg(long)]
        out: Option<PathBuf>,
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
        /// Text language: en, zh-hans, zh-hant or ja (0, 1, 2 also work).
        /// Default: the game's first language.
        #[arg(long)]
        lang: Option<String>,
        /// Font files for the message text, in fallback order. Without
        /// one, the text is left out of the frames.
        #[arg(long)]
        font: Vec<PathBuf>,
        /// Option to take at every selection.
        #[arg(long, default_value_t = 0)]
        choose: usize,
        /// Options for the first selections, in order, comma separated;
        /// later selections take --choose. The run stops when the game
        /// comes back to the title (an ending).
        #[arg(long, value_delimiter = ',')]
        path: Vec<usize>,
        /// きまぐれテンプテーション2: buttons to take at the first action
        /// menus, counted from 0, comma separated. Later menus take the
        /// buttons without a pink tick in turn. The first five menus and
        /// maps are saved as actionsN.png and mapN.png.
        #[arg(long, value_delimiter = ',')]
        actions: Vec<usize>,
        /// Height of the saved frames in pixels. The game is laid out in
        /// 1280 x 720; pictures are scaled up and text is drawn at this size.
        #[arg(long, default_value_t = 1080)]
        height: u32,
        /// How frames are drawn: `cpu` directly, `record` as the draw
        /// lists a GPU compositor gets, played back in software (to check
        /// them against `cpu`), `gpu` by that compositor (feature `gpu`).
        #[arg(long, default_value = "cpu")]
        renderer: String,
        /// Write the engine calls of the run to this trace file (see
        /// docs/trace.md).
        #[arg(long)]
        trace: Option<PathBuf>,
    },
    /// Compare two trace files and show where they part. Exits with 1 when
    /// they differ.
    Diff {
        /// The trace taken as right (the original program, or a run known
        /// to be good).
        expected: PathBuf,
        /// The trace checked against it.
        actual: PathBuf,
        /// Fields left out, comma separated: tick, script, pc, args,
        /// result. Against the original program, usually `tick,pc`.
        #[arg(long, default_value = "")]
        ignore: String,
        /// Compare only these calls: classes (Sprite), calls (Game.24) or
        /// kinds (message), comma separated.
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// Leave these calls out (same patterns as --only).
        #[arg(long, value_delimiter = ',')]
        skip: Vec<String>,
        /// Start each trace at its first call that matches, as `message`,
        /// `message:120` or `Script.0:FK001.MES` (call:first argument).
        #[arg(long)]
        from: Option<String>,
        /// Calls shown before each divergence.
        #[arg(long, default_value_t = 3)]
        context: usize,
        /// Divergences to show before stopping.
        #[arg(long, default_value_t = 1)]
        max: usize,
        /// How many calls of each trace to align at a divergence to find
        /// where they agree again.
        #[arg(long, default_value_t = 200)]
        window: usize,
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
            out,
        } => {
            let game = Game::open(&game)
                .with_context(|| format!("cannot open game folder {}", game.display()))?;
            let profile = Profile {
                push_big_endian: !push_le,
            };
            let language = pick_language(&silky_runtime::assets::languages(&game), lang)?;
            let options = TraceOptions {
                lang: language.map_section(),
                choose,
                max_calls,
                text_only,
                results: results.into_iter().collect(),
                out,
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
            path,
            actions,
            height,
            renderer,
            trace,
        } => {
            let game = Game::open(&game)
                .with_context(|| format!("cannot open game folder {}", game.display()))?;
            let language = pick_language(&silky_runtime::assets::languages(&game), lang)?;
            let picks = Picks {
                choose,
                path,
                actions,
                renderer,
            };
            let mut vm = Vm::new(Profile::default());
            if trace.is_some() {
                vm.trace_start();
            }
            let title = game.title;
            let result = snap(&mut vm, game, &out, &lines, language, &font, &picks, height);
            // The trace is written even when the run stops on an error: the
            // calls up to it are what shows the cause.
            if let Some(path) = trace {
                write_trace(&mut vm, "silky-run snap", title, &path)?;
            }
            result
        }
        Command::Diff {
            expected,
            actual,
            ignore,
            only,
            skip,
            from,
            context,
            max,
            window,
        } => {
            let read = |p: &PathBuf| {
                silky_trace::Trace::read(p).with_context(|| format!("reading {}", p.display()))
            };
            let (e, a) = (read(&expected)?, read(&actual)?);
            let projection = silky_trace::Projection {
                ignore: silky_trace::Fields::parse(&ignore).map_err(anyhow::Error::msg)?,
                only,
                skip,
                from,
            };
            let walk = silky_trace::Walk {
                context,
                max: max.max(1),
                window,
            };
            let report = silky_trace::diff(&e.calls, &a.calls, &projection, walk);
            print!("{report}");
            if !report.same() {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

/// Writes the calls `vm` recorded to a trace file. The command line goes
/// into its notes, without the game folder (a path of this computer).
fn write_trace(
    vm: &mut Vm,
    tool: &str,
    title: silky_format::game::Title,
    path: &std::path::Path,
) -> Result<()> {
    let mut t = silky_trace::Trace::runtime(tool);
    t.game = Some(format!("{title:?}"));
    let args: Vec<String> = std::env::args().skip(2).collect();
    // The first argument after the subcommand that is not an option is
    // the game folder.
    let mut skip_next = false;
    let mut folder_done = false;
    let mut kept = Vec::new();
    for a in args {
        if skip_next {
            skip_next = false;
            kept.push(a);
        } else if a.starts_with("--") {
            skip_next = !a.contains('=') && !matches!(a.as_str(), "--text-only" | "--push-le");
            kept.push(a);
        } else if !folder_done {
            folder_done = true;
            kept.push("<game>".into());
        } else {
            kept.push(a);
        }
    }
    t.notes.push(kept.join(" "));
    t.calls = vm.trace_take();
    t.write(path)
        .with_context(|| format!("writing {}", path.display()))?;
    println!("trace: {} calls in {}", t.calls.len(), path.display());
    Ok(())
}

/// What `snap` answers at selections and action menus.
struct Picks {
    choose: usize,
    path: Vec<usize>,
    actions: Vec<usize>,
    renderer: String,
}

#[allow(clippy::too_many_arguments)]
fn snap(
    vm: &mut Vm,
    game: Game,
    out: &std::path::Path,
    lines: &[u64],
    language: silky_runtime::Language,
    fonts: &[PathBuf],
    picks: &Picks,
    height: u32,
) -> Result<()> {
    let (choose, path) = (picks.choose, &picks.path);
    use silky_runtime::render::Renderer;
    use silky_runtime::text::TextRenderer;
    use silky_runtime::ui::{Layout, Ui};
    use silky_runtime::{Assets, Config, Pause, Runtime};

    std::fs::create_dir_all(out)?;
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
    match picks.renderer.as_str() {
        "cpu" => {}
        "record" => renderer.record = true,
        #[cfg(feature = "gpu")]
        "gpu" => {
            renderer.record = true;
            let c = silky_runtime::compose::Compositor::headless()
                .context("no GPU adapter for the compositor")?;
            COMPOSITOR.with(|g| *g.borrow_mut() = Some(c));
        }
        other => anyhow::bail!("unknown renderer {other} (cpu, record, or gpu with feature gpu)"),
    }
    let config = Config {
        language,
        ..Config::default()
    };
    renderer.set_title(game.title);
    renderer.scale = (height as f32 / renderer.size().1 as f32).max(1.0);
    let st = silky_runtime::system::Settings::for_title(game.title);
    renderer.window = (st.window_color, st.window_opacity);
    let assets = Assets::new(game);
    let layout = Layout::load(&assets, language)?;
    let mut rt = Runtime::new(assets, config);
    rt.boot(vm, "start.mes")?;
    let last = lines.iter().copied().max().unwrap_or(0);
    let mut line_no = 0u64;
    let mut title_saved = false;
    let mut selections = 0usize;
    let mut action_menus = 0usize;
    let mut maps = 0usize;
    let mut rooms = 0usize;
    let mut trains = 0usize;
    let mut towns = 0;
    let mut last_menu: (Vec<u32>, usize) = (Vec::new(), 0);
    // Frame shown when the game first waits after a movie (the movie's
    // still picture, the end card after the ending).
    let mut after_movie: Option<String> = None;
    let mut script = String::new();
    let mut polls = 0u64;
    loop {
        match vm.run(&mut rt, 50_000_000)? {
            Stop::Yield => match rt.pause.take() {
                Some(Pause::Title) => {
                    if !path.is_empty() && title_saved {
                        println!("back to the title after {line_no} lines, from {script}");
                        break;
                    }
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
                    // "Start" is the new game entry of the game's own menu.
                    let choice = match rt.config.title_choice {
                        1 => layout
                            .title_result(silky_runtime::ui::title::NEW_GAME)
                            .unwrap_or(1),
                        c => c,
                    };
                    rt.answer_title(vm, choice);
                }
                Some(Pause::Text { index }) => {
                    rt.jolts.clear();
                    rt.vo_steps.clear();
                    after_movie = None;
                    let name = rt.message_script();
                    if name != script {
                        script = name.to_string();
                        println!("script {script} at line {line_no}");
                    }
                    // Snow falls on for a second each message; the faces
                    // take their first step.
                    renderer.progress_snow(&rt.scene, 1000);
                    rt.tick_anime(1);
                    if lines.contains(&line_no) {
                        let mut frame = renderer.render(&rt.scene, &mut rt.assets)?;
                        let ui = Ui {
                            waiting: true,
                            plate: rt.plate(&vm),
                            jump_off: layout.is_kimaten2() && !rt.read_before,
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, true, None)?;
                        let script = rt.message_script().replace('.', "_");
                        let path = out.join(format!("line{line_no:05}-{script}-{index}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    if line_no >= last {
                        break;
                    }
                    line_no += 1;
                }
                // Headless: transitions and waits end at once; input polls
                // see a click now and then so wait-for-click loops end.
                Some(Pause::Transition(t)) => {
                    // A still picture held after a movie.
                    if let (Some(name), true) = (&after_movie, t.ms >= 1000) {
                        let frame = renderer.render(&rt.scene, &mut rt.assets)?;
                        let path = out.join(format!("after-{}.png", name.replace('.', "_")));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                        after_movie = None;
                    }
                    // The train of a live wait: a few frames of its way.
                    if t.live
                        && trains < 3
                        && rt.scene.train.as_ref().is_some_and(|x| x.moved.is_some())
                    {
                        trains += 1;
                        for pct in [0u32, 25, 50, 75] {
                            let mut scene = rt.scene.clone();
                            if let Some(train) = &mut scene.train {
                                train.progress(t.ms * pct / 100, t.ms);
                            }
                            let frame = renderer.render_scene(&scene, &mut rt.assets, false)?;
                            let path = out.join(format!("train{trains}-{pct:02}.png"));
                            write_png(&path, &frame)?;
                            println!("{}", path.display());
                        }
                    }
                    // The intro of the first start: a frame every few
                    // passes of its wait (and the last).
                    if t.live && rt.scene.intro.is_some() {
                        use silky_runtime::games::doukyuusei::intro::FRAMES;
                        for f in (0..FRAMES).step_by(16).chain([FRAMES]) {
                            let mut scene = rt.scene.clone();
                            if let Some(intro) = &mut scene.intro {
                                intro.frame = f;
                            }
                            let frame = renderer.render_scene(&scene, &mut rt.assets, false)?;
                            let path = out.join(format!("intro{f:03}.png"));
                            write_png(&path, &frame)?;
                            println!("{}", path.display());
                        }
                    }
                    // Second steps of effects end at once too.
                    while rt.follow_up() {
                        rt.pause = None;
                    }
                }
                Some(Pause::Frame) => {
                    if let Some(name) = &after_movie {
                        let frame = renderer.render(&rt.scene, &mut rt.assets)?;
                        let path = out.join(format!("after-{}.png", name.replace('.', "_")));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                        after_movie = None;
                    }
                    polls += 1;
                    if polls % 600 == 0 {
                        rt.clicked = true;
                    }
                }
                // Headless: H mode is left at once with its finish button.
                Some(Pause::HMode) => println!("H mode (finished at once)"),
                Some(Pause::Choice) => {
                    let choose = path.get(selections).copied().unwrap_or(choose);
                    selections += 1;
                    println!("select [{}] -> {}", rt.choices.join(" | "), choose);
                    let pick = choose.min(rt.choices.len().saturating_sub(1));
                    rt.answer_choice(vm, pick);
                }
                Some(Pause::Actions) => {
                    if action_menus < 5 {
                        let mut frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let ui = Ui {
                            actions: rt.actions.clone(),
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                        let path = out.join(format!("actions{action_menus}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    // Buttons not done yet (no pink tick), taken in turn
                    // while the same menu comes back.
                    let icons: Vec<u32> = rt.actions.iter().map(|a| a.icon).collect();
                    if last_menu.0 == icons {
                        last_menu.1 += 1;
                    } else {
                        last_menu = (icons, 0);
                    }
                    let open: Vec<_> = rt.actions.iter().filter(|a| a.tick != 2).collect();
                    let turn = match open.len() {
                        0 => rt.actions.first(),
                        n => Some(open[last_menu.1 % n]),
                    };
                    let pick = picks
                        .actions
                        .get(action_menus)
                        .and_then(|&i| rt.actions.get(i))
                        .or(turn)
                        .map_or(-1, |a| a.result);
                    println!("actions {:?} -> {pick}", last_menu.0);
                    action_menus += 1;
                    rt.answer_action(vm, pick);
                }
                Some(Pause::Map) => {
                    use silky_runtime::games::kimaten2::map::{Click, BACK};
                    let mut m = rt.map.clone().unwrap();
                    // The first place not done yet, through its panel on
                    // the second map; else back.
                    let mut pick = BACK;
                    for i in 0..m.places() {
                        let spot = m.place(i);
                        if !spot.shown || spot.done {
                            continue;
                        }
                        match m.click_place(i) {
                            Some(Click::Result(r)) => pick = r,
                            Some(Click::OpenPanel(p)) => {
                                m.panel = Some(p);
                                let (spots, _) = m.panel_spots(p);
                                match spots.iter().find(|(_, s)| s.enabled && !s.done) {
                                    Some((k, _)) => match m.click_panel(*k) {
                                        Some(Click::Result(r)) => pick = r,
                                        _ => continue,
                                    },
                                    None => continue,
                                }
                            }
                            _ => continue,
                        }
                        break;
                    }
                    if maps < 5 {
                        let mut frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let ui = Ui {
                            map: Some(m.clone()),
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                        let path = out.join(format!("map{maps}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    println!("map {} -> {pick}", if m.second { 2 } else { 1 });
                    maps += 1;
                    rt.answer_action(vm, pick);
                }
                Some(Pause::Panel)
                    if matches!(
                        rt.panel,
                        Some(silky_runtime::games::doukyuusei::Panel::Town)
                    ) =>
                {
                    // The walk on the map: the frame as it opens, then the
                    // shortest way to the nearest place at least a few
                    // steps away.
                    use silky_runtime::games::doukyuusei::town::Input;
                    if towns < 5 {
                        let mut frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let ui = Ui {
                            panel: rt.panel.clone(),
                            plate: rt.plate(&vm),
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                        let path = out.join(format!("town{towns}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    let keys = rt
                        .town
                        .as_ref()
                        .and_then(|t| route_to_place(t, 4, towns))
                        .unwrap_or_default();
                    let mut found = None;
                    for (k, &key) in keys.iter().chain([0u8; 8].iter()).enumerate() {
                        let input = Input {
                            keys: key,
                            ..Default::default()
                        };
                        if let Some(id) = rt.town_tick(vm, &input) {
                            found = Some(id);
                            break;
                        }
                        let _ = k;
                    }
                    let Some(id) = found else {
                        let frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        write_png(&out.join("town_stuck.png"), &frame)?;
                        let h = rt.town.as_ref().map(|t| t.walkers[0]);
                        anyhow::bail!("no place reached on the map: {h:?}");
                    };
                    if towns < 5 {
                        let frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let path = out.join(format!("town{towns}_end.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                    }
                    println!(
                        "town -> {id} (day {:?} time {:?})",
                        vm.var_g(7),
                        vm.var_g(19)
                    );
                    towns += 1;
                    rt.answer_action(vm, id);
                }
                Some(Pause::Panel) => {
                    use silky_runtime::games::doukyuusei::{room::THING, Click, Panel};
                    let mut panel = rt.panel.clone().unwrap();
                    let (name, pick) = match &mut panel {
                        // Each thing in turn but the bed (the last) at
                        // home, then talk (away from home), then out.
                        Panel::Room(r) => {
                            use silky_runtime::games::doukyuusei::room::{EXIT, SLEEP};
                            let mut things = r.things();
                            if r.set() == 0 {
                                things.pop();
                            }
                            let mut ids: Vec<i32> = things.iter().map(|&t| THING + t).collect();
                            // The plain area selection: things only.
                            if r.is_plain() {
                                let pick = match ids.is_empty() {
                                    true => Click::Result(0),
                                    false => r.click(ids[rooms % ids.len()]),
                                };
                                ("area", pick)
                            } else {
                                if r.set() != 0 {
                                    ids.push(SLEEP as i32);
                                }
                                ids.push(EXIT as i32);
                                let pick = r.click(ids[rooms % ids.len()]);
                                ("room", pick)
                            }
                        }
                        // One hour.
                        Panel::Hours(h) => ("hours", Click::Result(h.click(0))),
                        // The first girl that can be picked, else back.
                        Panel::Picker(p) => {
                            use silky_runtime::games::doukyuusei::picker::{BACK, CARDS};
                            let card = (0..CARDS as i32).find(|&i| p.enabled(i));
                            let pick = match (card, p.sub) {
                                (Some(i), _) => p.click(i),
                                (None, 0) => p.click(BACK),
                                (None, _) => Click::Result(1),
                            };
                            ("picker", pick)
                        }
                        // Leave.
                        Panel::Album(a) => {
                            use silky_runtime::games::doukyuusei::album::BACK;
                            ("album", a.click(BACK))
                        }
                        // Leave.
                        Panel::Reel(r) => {
                            use silky_runtime::games::doukyuusei::reel::BACK;
                            ("reel", r.click(BACK))
                        }
                        // Leave.
                        Panel::Music(m) => {
                            use silky_runtime::games::doukyuusei::music::BACK;
                            ("music", m.click(BACK))
                        }
                        // Leave.
                        Panel::Comment(c) => {
                            use silky_runtime::games::doukyuusei::comment::BACK;
                            ("comment", c.click(BACK))
                        }
                        // Confirm the default name.
                        Panel::Name(n) => {
                            use silky_runtime::games::doukyuusei::name::CONFIRM;
                            ("name", n.click(CONFIRM))
                        }
                        // Close it.
                        Panel::Notice(n) => {
                            use silky_runtime::games::doukyuusei::notice::CLOSE;
                            ("notice", n.click(CLOSE))
                        }
                        // Close it.
                        Panel::Unlocked(u) => {
                            use silky_runtime::games::doukyuusei::unlocked::BACK;
                            ("unlocked", u.click(BACK))
                        }
                        Panel::Town | Panel::Jump(_) => unreachable!(),
                    };
                    let pick = match pick {
                        Click::Result(pick) => pick,
                        Click::Name | Click::Close => 0,
                        _ => unreachable!(),
                    };
                    if rooms < 5
                        || name == "picker"
                        || name == "album"
                        || name == "reel"
                        || name == "music"
                        || name == "comment"
                        || name == "name"
                        || name == "notice"
                        || name == "unlocked"
                    {
                        let mut frame = renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                        let ui = Ui {
                            panel: Some(panel.clone()),
                            plate: rt.plate(&vm),
                            ..Ui::default()
                        };
                        layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                        let path = out.join(format!("{name}{rooms}.png"));
                        write_png(&path, &frame)?;
                        println!("{}", path.display());
                        // The first room again with its menu open.
                        if rooms == 0 {
                            let mut frame =
                                renderer.render_scene(&rt.scene, &mut rt.assets, false)?;
                            let ui = Ui {
                                side_menu: true,
                                ..ui
                            };
                            layout.draw(&mut frame, &mut rt.assets, &ui, false, None)?;
                            let path = out.join("room0_menu.png");
                            write_png(&path, &frame)?;
                            println!("{}", path.display());
                        }
                    }
                    println!("{name} -> {pick}");
                    rooms += 1;
                    match &panel {
                        Panel::Name(n) => rt.finish_name(vm, &n.text()),
                        Panel::Notice(_) | Panel::Unlocked(_) => rt.panel = None,
                        _ => rt.answer_action(vm, pick),
                    }
                }
                // Headless: leave extras screens, movies end at once.
                Some(Pause::Extras(_)) => rt.answer_extras(vm, -1),
                Some(Pause::Movie) => {
                    println!("movie {}", rt.movie.as_deref().unwrap_or("?"));
                    after_movie = rt.movie.clone();
                    rt.answer_movie(vm, true)
                }
                None => {}
            },
            Stop::Options => {
                let numbers: Vec<i32> = vm.options().iter().map(|(_, v)| v.int()).collect();
                rt.begin_options(&numbers);
                vm.run_all_options()?;
            }
            Stop::Ended => {
                println!("end of scripts in {}", vm.script_name().unwrap_or("?"));
                break;
            }
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

#[cfg(feature = "gpu")]
thread_local! {
    static COMPOSITOR: std::cell::RefCell<Option<silky_runtime::compose::Compositor>> =
        const { std::cell::RefCell::new(None) };
}

/// Writes a frame at its output size (see `Canvas::present`).
fn write_png(path: &std::path::Path, frame: &silky_runtime::render::Canvas) -> Result<()> {
    #[cfg(feature = "gpu")]
    let gpu = COMPOSITOR.with(|g| g.borrow_mut().as_mut().and_then(|c| c.render_rgba(frame)));
    #[cfg(not(feature = "gpu"))]
    let gpu = None;
    let frame = &gpu.unwrap_or_else(|| frame.present());
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, frame.width, frame.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&frame.rgba)?;
    Ok(())
}

fn load_script(game: &Game, name: &str) -> Result<Script> {
    game.script(name)
        .with_context(|| format!("cannot read script {name}"))
}

/// The language asked for, which the game must have, or its first one.
fn pick_language(
    available: &[silky_runtime::Language],
    asked: Option<String>,
) -> Result<silky_runtime::Language> {
    let Some(name) = asked else {
        return Ok(available[0]);
    };
    let lang = silky_runtime::Language::parse(&name)
        .with_context(|| format!("unknown language {name:?}: use en, zh-hans, zh-hant or ja"))?;
    anyhow::ensure!(
        available.contains(&lang),
        "this game has no {lang:?} text (it has {available:?})"
    );
    Ok(lang)
}

fn parse_result(s: &str) -> std::result::Result<((i32, i32), Vec<i32>), String> {
    let err = || format!("expected CLASS.SUB=VALUE, got {s:?}");
    let (key, value) = s.split_once('=').ok_or_else(err)?;
    let (class, sub) = key.split_once('.').ok_or_else(err)?;
    let num = |t: &str| t.trim().parse::<i32>().map_err(|_| err());
    let values = value
        .split(',')
        .map(num)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(((num(class)?, num(sub)?), values))
}

struct TraceOptions {
    lang: usize,
    choose: usize,
    max_calls: u64,
    text_only: bool,
    /// Values still to return, by call; the last one stays.
    results: HashMap<(i32, i32), Vec<i32>>,
    /// Trace file to write.
    out: Option<PathBuf>,
}

fn trace(game: Game, profile: Profile, start: &str, options: TraceOptions) -> Result<()> {
    let max_calls = options.max_calls;
    let mut vm = Vm::new(profile);
    if options.out.is_some() {
        vm.trace_start();
    }
    let title = game.title;
    vm.jump_script(start, load_script(&game, start)?);
    let mut host = TraceHost {
        game,
        map: None,
        calls: 0,
        options,
    };
    let result = (|| -> Result<()> {
        loop {
            let stop = vm.run(&mut host, 1_000_000)?;
            match stop {
                Stop::Yield => {}
                Stop::Options => {
                    // The option bodies register labels; the selection
                    // itself is Game.20, answered with --choose.
                    println!("options: {}", vm.options().len());
                    vm.run_all_options()?;
                }
                Stop::Ended => {
                    println!("end of script after {} instructions", vm.steps);
                    return Ok(());
                }
                Stop::Budget => {}
            }
            if host.calls >= max_calls {
                println!(
                    "stopped after {} engine calls ({} instructions)",
                    host.calls, vm.steps
                );
                return Ok(());
            }
        }
    })();
    if let Some(path) = host.options.out.take() {
        write_trace(&mut vm, "silky-run trace", title, &path)?;
    }
    result
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

    fn result(&mut self, vm: &mut Vm, class: i32, sub: i32) {
        if let Some(values) = self.options.results.get_mut(&(class, sub)) {
            let v = if values.len() > 1 {
                values.remove(0)
            } else {
                values[0]
            };
            vm.push(v);
        } else if (class, sub) == (GAME, 20) {
            println!("select -> option {}", self.options.choose);
            vm.push(self.options.choose as i32 + 1);
        } else if native::returns_value(self.game.title, &self.game.engine, class, sub) {
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
                if let (11, [Value::Str(speaker)], true) =
                    (cmd, args.as_slice(), self.options.text_only)
                {
                    println!("{:>12}  {speaker}", "speaker");
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
                // Japanese builds: the message text is in the script and
                // goes to BackLog.3 before MesWnd.3 shows it.
                if (class, sub) == (BACKLOG, 3) && self.options.text_only {
                    if let [Value::Str(text)] = args.as_slice() {
                        println!("{:>12}  {}", "text", text.replace('\n', " / "));
                        return Ok(Flow::Continue);
                    }
                }
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

/// Arrow keys, a step each, that take the hero of a walking map into a
/// place at least `min` steps away: the `pick`-th nearest (counting
/// around), breadth first and without the sliding along walls.
fn route_to_place(
    t: &silky_runtime::games::doukyuusei::town::Town,
    min: usize,
    pick: usize,
) -> Option<Vec<u8>> {
    use silky_runtime::games::doukyuusei::town::dir;
    use std::collections::{HashMap, VecDeque};
    let hero = t.walkers.iter().position(|w| w.flags & 0x40 != 0)?;
    let h = t.walkers[hero];
    let (mw, mh) = t.size();
    let (bw, bh) = (h.w as i32, h.h as i32);
    let places = t.places();
    let start = (h.x as i32, h.y as i32);
    let mut came: HashMap<(i32, i32), ((i32, i32), u8)> = HashMap::new();
    let mut depth: HashMap<(i32, i32), usize> = HashMap::from([(start, 0)]);
    let mut queue = VecDeque::from([start]);
    let mut found: Vec<(u16, (i32, i32))> = Vec::new();
    // Key, facing bit of the place table.
    let moves = [
        (dir::UP, 1u16, 0, -1),
        (dir::DOWN, 2, 0, 1),
        (dir::LEFT, 4, -1, 0),
        (dir::RIGHT, 8, 1, 0),
    ];
    while let Some((x, y)) = queue.pop_front() {
        let d = depth[&(x, y)];
        for &(key, bit, dx, dy) in &moves {
            let blocked = match key {
                dir::UP => y == 0 || (0..bw).any(|k| t.wall(x + k, y)),
                dir::DOWN => y + bh >= mh || (0..bw).any(|k| t.wall(x + k, y + bh)),
                dir::LEFT => x == 0 || (1..bh).any(|k| t.wall(x - 1, y + k)),
                _ => x + bw >= mw || (1..bh).any(|k| t.wall(x + bw, y + k)),
            };
            if blocked {
                continue;
            }
            let n = (x + dx, y + dy);
            if depth.contains_key(&n) {
                continue;
            }
            depth.insert(n, d + 1);
            came.insert(n, ((x, y), key));
            let inside = places.iter().find(|&&(_, l, tp, r, b, f)| {
                n.0 <= r as i32
                    && n.0 + bw > l as i32
                    && n.1 + 1 <= b as i32
                    && n.1 + bh > tp as i32
                    && f & bit != 0
            });
            match inside {
                Some(p) if d + 1 >= min => {
                    if !found.iter().any(|f| f.0 == p.0) {
                        found.push((p.0, n));
                    }
                }
                // Walking through another place would end the walk there.
                Some(_) => {}
                None => queue.push_back(n),
            }
        }
    }
    let &(_, mut at) = found.get(pick % found.len().max(1))?;
    let mut keys = Vec::new();
    while let Some(&(from, key)) = came.get(&at) {
        keys.push(key);
        at = from;
    }
    keys.reverse();
    Some(keys)
}
