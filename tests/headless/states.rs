//! Engine of the state tests (`state_*.rs`): per control, the pixels at rest, hovered and held down equal the state table's role
//! value over the control's backdrop, light and dark. A spec is one row of the table; a failure names the control and the state.
#![allow(dead_code)]
use super::*;
use gpui_calendar::theme::contrast::over;
use gpui_kit::test::{TestEventExt, TestQueryExt};
use gpui_kit::{Bounds, MouseButton, Pixels, point, px, size};

pub type Role = fn(&Theme) -> Rgb;
type Exp = Box<dyn Fn(&Theme, Rgb) -> Rgb>;
type Zone = fn(Bounds<Pixels>) -> Bounds<Pixels>;
type Specs = Box<dyn Fn(&mut Win) -> Vec<Spec>>;

/// Where a control is sampled: its flat fill (most common color inside), a point `dx` px in from the left edge, or its top outline.
#[derive(Clone, Copy)]
pub enum Probe {
    Fill,
    Left(f32),
    Top,
}

/// What holding the pointer down does. `Fill`: the probe reads a new value. `Dim`: the 85% dim, measured as a margin (a dim of a
/// near-equal fill moves a pixel by under TOL, so a value check could not fail); the backdrop is a role, or read just outside the box.
/// `Same`: nothing changes inside the zone (default: the whole box).
enum Press {
    None,
    Fill(Exp),
    Dim(Option<Role>),
    Same(Option<Zone>),
}

pub struct Spec {
    pub id: String,
    pub probe: Probe,
    rest: Exp,
    hover: Option<Exp>,
    press: Press,
    ring: Role,
    focus: bool,
    ringed: bool,
}

fn lit(r: Role) -> Exp {
    Box::new(move |t, _| r(t))
}

fn spec(id: &str, probe: Probe, rest: Exp, hover: Option<Exp>, press: Press) -> Spec {
    Spec { id: id.into(), probe, rest, hover, press, ring: |t| t.ring, focus: true, ringed: true }
}

impl Spec {
    /// Ghost: `fg` at 6% over its fill (`wash-hover`); pressed is that fill at 85% over what is behind the box.
    pub fn ghost(id: &str, fill: Role, behind: Role) -> Spec {
        Spec::ghost_by(id, fill, behind, |t| t.neutral.t1)
    }

    pub fn ghost_by(id: &str, fill: Role, behind: Role, fg: Role) -> Spec {
        let hot = move |t: &Theme| over(fg(t), 0.06, fill(t));
        spec(id, Probe::Fill, lit(fill), Some(Box::new(move |t, _| hot(t))), Press::Dim(Some(behind)))
    }

    /// List row: `wash-hover`, then `wash-press` (a fill, not a dim).
    pub fn row(id: &str, fill: Role) -> Spec {
        spec(id, Probe::Fill, lit(fill), Some(Box::new(move |t, _| over(t.neutral.t1, 0.06, fill(t)))), Press::Fill(Box::new(move |t, _| over(t.neutral.t1, 0.10, fill(t)))))
    }

    /// Filled ink: 90% then 80% of `ink` over `page`.
    pub fn filled(id: &str) -> Spec {
        spec(id, Probe::Fill, lit(|t| t.ink), Some(Box::new(|t, _| over(t.ink, 0.9, t.neutral.page))), Press::Fill(Box::new(|t, _| over(t.ink, 0.8, t.neutral.page))))
    }

    /// A tinted chip: `wash-hover` over its tint.
    pub fn chip(id: &str, tint: Role, behind: Role) -> Spec {
        let hot = move |t: &Theme| over(t.neutral.t1, 0.06, tint(t));
        spec(id, Probe::Fill, lit(tint), Some(Box::new(move |t, _| hot(t))), Press::Dim(Some(behind)))
    }

    /// A radio card: the outline goes `border` to `t2`; pressed dims.
    pub fn card(id: &str, behind: Role) -> Spec {
        spec(id, Probe::Top, lit(|t| t.neutral.border), Some(lit(|t| t.neutral.t2)), Press::Dim(Some(behind)))
    }

    /// Selected: hover changes nothing, pressed dims.
    pub fn selected(id: &str, fill: Role, behind: Role) -> Spec {
        spec(id, Probe::Fill, lit(fill), Some(lit(fill)), Press::Dim(Some(behind)))
    }

    /// Banner button: a black scrim of 28% at rest, 40% hovered, over the gradient (recovered from the rest sample).
    pub fn banner(id: &str) -> Spec {
        let g = |s: Rgb| {
            let k = |v: u8| f32::from(v) / 0.72;
            (k(s.0), k(s.1), k(s.2))
        };
        let mix = move |s: Rgb, keep: f32, from: Rgb| {
            let (r, gr, b) = g(s);
            let f = |x: f32, base: u8| (x * keep + f32::from(base) * (1. - keep)).round().clamp(0., 255.) as u8;
            Rgb(f(r, from.0), f(gr, from.1), f(b, from.2))
        };
        let hover = move |s: Rgb| mix(s, 0.60, Rgb(0, 0, 0));
        spec(id, Probe::Left(5.), Box::new(|_, s| s), Some(Box::new(move |_, s| hover(s))), Press::Dim(None)).ring_by(|t| t.banner.value)
    }

    /// A fill that does not change: rest only.
    pub fn fixed(id: &str, fill: Role) -> Spec {
        spec(id, Probe::Fill, lit(fill), None, Press::None).nofocus()
    }

    /// A month day cell: `wash-hover`, and no press at all: the cell is a container, so only the date number is watched for a dim.
    pub fn cell(id: &str, fill: Role) -> Spec {
        let mut s = spec(id, Probe::Fill, lit(fill), Some(Box::new(move |t, _| over(t.neutral.t1, 0.06, fill(t)))), Press::Same(Some(number_box)));
        s.focus = false;
        s
    }

    /// Disabled: hover and press change nothing (the probe and the whole box).
    pub fn disabled(id: &str, probe: Probe) -> Spec {
        spec(id, probe, Box::new(|_, s| s), Some(Box::new(|_, s| s)), Press::Same(None)).nofocus()
    }

    /// The ring is clipped at the box edge (a full-width row in the sidebar), so only the fill is checked under focus.
    pub fn noring(mut self) -> Spec {
        self.ringed = false;
        self
    }

    pub fn nofocus(mut self) -> Spec {
        self.focus = false;
        self
    }

    pub fn ring_by(mut self, r: Role) -> Spec {
        self.ring = r;
        self
    }

    pub fn at(mut self, p: Probe) -> Spec {
        self.probe = p;
        self
    }
}

/// One window to open (a view, its events, an optional setup) and the controls it must show.
pub struct Group {
    pub view: View,
    pub events: Vec<Event>,
    pub setup: Box<dyn Fn(&mut Win)>,
    pub specs: Specs,
}

impl Group {
    pub fn new(view: View, events: Vec<Event>, specs: impl Fn(&mut Win) -> Vec<Spec> + 'static) -> Group {
        Group { view, events, setup: Box::new(|_| {}), specs: Box::new(specs) }
    }

    pub fn setup(mut self, f: impl Fn(&mut Win) + 'static) -> Group {
        self.setup = Box::new(f);
        self
    }
}

pub fn locate(win: &mut Win, id: &str) -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    // `name*` matches every id that starts with `name`; a bare name matches that id only.
    let key = id.strip_suffix('*').map_or(format!("\"{id}\""), |p| format!("\"{p}"));
    win.cx.update_window(win.h, |_, w, _| w.elements().into_iter().find(|e| e.path().last().is_some_and(|p| format!("{p:?}").contains(&key))).map(|e| e.bounds())).unwrap()
}

fn sample(win: &mut Win, b: gpui_kit::Bounds<gpui_kit::Pixels>, probe: Probe) -> Rgb {
    sample_in(&win.shot(), b, probe)
}

fn sample_in(shot: &Shot, b: gpui_kit::Bounds<gpui_kit::Pixels>, probe: Probe) -> Rgb {
    let (cx, cy) = (f32::from(b.center().x), f32::from(b.center().y));
    match probe {
        Probe::Fill => mode_of(&shot.region(b, 3.)),
        Probe::Left(dx) => shot.mean(f32::from(b.left()) + dx, cy, 0.8),
        Probe::Top => shot.mean(cx, f32::from(b.top()) + 0.5, 0.4),
    }
}

/// The 22px date-number box of a month cell: top border 1 + pt 6, right px 4 + 6.
pub fn number_box(c: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds { origin: point(c.right() - px(32.), c.top() + px(7.)), size: size(px(22.), px(22.)) }
}

/// Holding the pointer on `inner` (a leaf inside the container `outer`) must leave `outer`'s date number as it was: only the leaf dims.
/// Returns how far the number's pixels moved.
pub fn leaf_press_moves_container(win: &mut Win, outer: &str, inner: &str) -> u8 {
    let (o, i) = (locate(win, outer).expect("container"), locate(win, inner).expect("leaf"));
    win.act(|w, cx| w.pointer_move(i.center(), None, cx));
    let before = win.shot().region(number_box(o), 0.);
    win.act(|w, cx| w.pointer_down(i.center(), MouseButton::Left, cx));
    let held = win.shot().region(number_box(o), 0.);
    win.act(|w, cx| w.pointer_up(point(px(2.), px(2.)), MouseButton::Left, cx));
    spread(&before, &held)
}

/// Largest per-channel difference between two equal-sized pixel lists.
fn spread(a: &[Rgb], b: &[Rgb]) -> u8 {
    a.iter().zip(b).map(|(&x, &y)| delta(x, y)).max().unwrap_or(0)
}

const TOL: u8 = 3;
/// A dim must move the pixels by at least this share of what a true 85% would, and no more than this multiple (a double dim is 1.9x).
const DIM_MIN: f32 = 0.5;
const DIM_MAX: f32 = 1.5;
/// Below this the control has no pixel that contrasts with what is behind it, so a dim could not be seen: the spec proves nothing.
const DIM_FLOOR: f32 = 5.;
const TABS: usize = 150;

/// Press `key` until `id` has keyboard focus (the pointer stays where it is, so hover would stay on if the framework allowed it).
/// False if never reached, or if focus stops moving (a text box that keeps Tab).
fn tab_to(win: &mut Win, id: &str, key: &str) -> bool {
    let (mut last, mut same) = (None, 0);
    for _ in 0..TABS {
        win.cx.update_window(win.h, |_, w, cx| w.press(key, cx)).unwrap();
        win.cx.update_window(win.h, |_, w, cx| w.render_frame(cx)).unwrap();
        let (hit, now) = win.cx.update_window(win.h, |_, w, _| (w.try_find(id.to_string()).is_some_and(|e| e.focused() == Some(true)), w.elements().into_iter().rfind(|e| e.focused() == Some(true)).map(|e| format!("{:?}", e.path().last())))).unwrap();
        if hit {
            win.render();
            return true;
        }
        same = if now == last { same + 1 } else { 0 };
        if same >= 3 {
            return false;
        }
        last = now;
    }
    false
}

/// Run every group in light and dark; panic with one line per mismatch. Writes artifacts/verify/states-`area`.txt.
pub fn run(area: &str, groups: Vec<Group>) {
    let away = point(px(2.), px(2.));
    let (mut rows, mut fails, mut checks) = (vec![], vec![], 0);
    for mode in [Mode::Light, Mode::Dark] {
        for g in &groups {
            let mut win = open(mode, Neutral::Cool, g.view, g.events.clone(), (1440., 900.));
            (g.setup)(&mut win);
            let theme = win.theme;
            let specs = (g.specs)(&mut win);
            if specs.is_empty() {
                fails.push(format!("{mode:?} {area}: a group listed no specs"));
            }
            for s in specs {
                let Some(b) = locate(&mut win, &s.id) else {
                    fails.push(format!("{mode:?} {area} {}: control not found", s.id));
                    continue;
                };
                let mut log = |state: &str, ok: bool, what: String| {
                    checks += 1;
                    rows.push(format!("{:<6} {:<28} {:<10} {what} {}", format!("{mode:?}"), s.id, state, if ok { "ok" } else { "FAIL" }));
                    if !ok {
                        fails.push(format!("{mode:?} {area} {} {state}: {what}", s.id));
                    }
                };
                let value = |log: &mut dyn FnMut(&str, bool, String), state: &str, got: Rgb, want: Rgb| log(state, delta(got, want) <= TOL, format!("got {got:?} want {want:?}"));
                win.act(|w, cx| w.pointer_move(away, None, cx));
                let rest = sample(&mut win, b, s.probe);
                value(&mut log, "rest", rest, (s.rest)(&theme, rest));
                let c = b.center();
                win.act(|w, cx| w.pointer_move(c, None, cx));
                let hover_shot = win.shot();
                if let Some(h) = &s.hover {
                    value(&mut log, "hover", sample_in(&hover_shot, b, s.probe), h(&theme, rest));
                }
                if !matches!(s.press, Press::None) {
                    win.act(|w, cx| w.pointer_down(c, MouseButton::Left, cx));
                    let held_shot = win.shot();
                    win.act(|w, cx| w.pointer_up(away, MouseButton::Left, cx));
                    let zone = match &s.press {
                        Press::Same(Some(z)) => z(b),
                        _ => b,
                    };
                    let inset = if matches!(s.press, Press::Same(Some(_))) { 0. } else { 1. };
                    let (before, held) = (hover_shot.region(zone, inset), held_shot.region(zone, inset));
                    let moved = f32::from(spread(&before, &held));
                    match &s.press {
                        Press::Fill(p) => value(&mut log, "pressed", sample_in(&held_shot, b, s.probe), p(&theme, rest)),
                        Press::Dim(behind) => {
                            let behind = behind.map_or(hover_shot.at(f32::from(b.left()) - 4., f32::from(c.y)), |r| r(&theme));
                            let want = 0.15 * before.iter().map(|&p| f32::from(delta(p, behind))).fold(0., f32::max);
                            log("pressed", want >= DIM_FLOOR && moved >= DIM_MIN * want && moved <= DIM_MAX * want + f32::from(TOL), format!("moved {moved} of a true 85% dim {want:.1}"));
                        }
                        Press::Same(_) => log("pressed", moved <= f32::from(TOL), format!("moved {moved}")),
                        Press::None => {}
                    }
                    win.act(|w, cx| w.pointer_move(away, None, cx));
                }
                if s.focus {
                    // Keyboard focus while the pointer is on the control: the ring wins, the fill stays at rest, and a press still shows.
                    // A fresh window per direction, so earlier presses leave no focus behind; the form's notes box swallows Tab, so its
                    // footer is reached backwards.
                    let fresh = || {
                        let mut f = open(mode, Neutral::Cool, g.view, g.events.clone(), (1440., 900.));
                        (g.setup)(&mut f);
                        f.act(|w, cx| w.pointer_move(c, None, cx));
                        f
                    };
                    let reached = ["tab", "shift-tab"].into_iter().find_map(|k| Some(fresh()).and_then(|mut f| tab_to(&mut f, &s.id, k).then_some(f)));
                    if let Some(mut win) = reached {
                        let shot = win.shot();
                        value(&mut log, "focus+hover", sample_in(&shot, b, s.probe), rest);
                        let ring = shot.mean(f32::from(b.left()) - 3., f32::from(c.y), 0.4);
                        log("ring", !s.ringed || delta(ring, (s.ring)(&theme)) <= 2 * TOL, format!("got {ring:?} want {:?}", (s.ring)(&theme)));
                        if let Press::Fill(p) = &s.press {
                            win.act(|w, cx| w.pointer_down(c, MouseButton::Left, cx));
                            value(&mut log, "focus+press", sample(&mut win, b, s.probe), p(&theme, rest));
                            win.act(|w, cx| w.pointer_up(away, MouseButton::Left, cx));
                        }
                    } else {
                        log("focus", false, format!("not reached by {TABS} tabs"));
                    }
                }
                win.act(|w, cx| w.pointer_move(away, None, cx));
            }
        }
    }
    artifact(&format!("states-{area}.txt"), &(rows.join("\n") + "\n"));
    assert!(fails.is_empty(), "{} of {checks} state checks failed:\n{}", fails.len(), fails.join("\n"));
    println!("{area} states ok: {checks} checks");
}
