// ─────────────────────────────────────────────────────────────────────────────
// DIPRALIX ◈ NULLVOID Terminal Theme
// Phantom-Hacker Aesthetic — "Wraith-Core Engine"
//
// Concept: spectral echoes of computation; phosphor light in absolute dark.
// No emoji — pure Unicode geometric & technical glyphs.
//
// Drop this file into src/ui/nullvoid.rs and call print_banner() at startup.
// ─────────────────────────────────────────────────────────────────────────────

#![allow(dead_code)]

use std::io::{self, Write};

// ══════════════════════════════════════════════════════════════════════════════
// ANSI primitives
// ══════════════════════════════════════════════════════════════════════════════

pub const RESET:  &str = "\x1b[0m";
pub const BOLD:   &str = "\x1b[1m";
pub const DIM:    &str = "\x1b[2m";
const CLRLN:      &str = "\x1b[2K"; // erase entire current line

fn cursor_up(n: usize) -> String   { format!("\x1b[{}A", n) }
fn cursor_col_1() -> &'static str  { "\x1b[1G" } // move to col 1

// ══════════════════════════════════════════════════════════════════════════════
// NULLVOID Palette (RGB true-color foreground)
// ══════════════════════════════════════════════════════════════════════════════
//
// ◈ Phosphor Mint  #3DFF9A  → status, success, A-letter
// ◈ Molten Fire    #FF5C1A  → dipralix heat, errors, D-letter accent
// ◈ Plasma Cyan    #00D4FF  → data streams, info, I-letter
// ◈ Void Violet    #7B51FF  → cipher band, headers, R-letter
// ◈ Void Amber     #FFB86C  → warnings, models, filenames
// ◈ Bright         #CDD6F4  → primary output text
// ◈ Body Text      #8892B0  → secondary text
// ◈ Muted          #3A4060  → decorations, rules
// ◈ Spectral Ghost #141C38  → the phantom echo behind the logo
// ──────────────────────────────────────────────────────────────────────────────

pub const MINT:   &str = "\x1b[38;2;61;255;154m";
pub const FIRE:   &str = "\x1b[38;2;255;92;26m";
pub const PLASMA: &str = "\x1b[38;2;0;212;255m";
pub const VIOLET: &str = "\x1b[38;2;123;81;255m";
pub const AMBER:  &str = "\x1b[38;2;255;184;108m";
pub const BRIGHT: &str = "\x1b[38;2;205;214;244m";
pub const TEXT:   &str = "\x1b[38;2;136;146;176m";
pub const MUTED:  &str = "\x1b[38;2;58;64;96m";
pub const GHOST:  &str = "\x1b[38;2;18;24;52m";

// ══════════════════════════════════════════════════════════════════════════════
// NULLVOID Icon Set (Unicode, never emoji)
// ══════════════════════════════════════════════════════════════════════════════

pub const I_MARK:   &str = "◈";  // dipralix mark / status
pub const I_TARGET: &str = "⌖";  // crosshair / analysis / secure
pub const I_PROC:   &str = "⎔";  // processor / settings
pub const I_GRID:   &str = "⊞";  // files / context grid
pub const I_BRANCH: &str = "⟟";  // git branch / path
pub const I_PROMPT: &str = "⊢";  // logical turnstile / prompt arrow
pub const I_OUT:    &str = "◆";  // output diamond / reasoning
pub const I_STREAM: &str = "∷";  // data stream / proportional
pub const I_WARN:   &str = "⌬";  // delta / warning / change
pub const I_ACTIVE: &str = "⊛";  // active / starred
pub const I_ERROR:  &str = "⊗";  // error / tensor product
pub const I_ADD:    &str = "⊕";  // addition / ok / loaded
pub const I_LIVE:   &str = "◉";  // live / filled circle
const I_HALF:   &str = "◐";  // half circle (thinking)
const I_HALF2:  &str = "◓";  // spinner frame 2
const I_HALF3:  &str = "◑";  // spinner frame 3
const I_HALF4:  &str = "◒";  // spinner frame 4
const I_RULE:   &str = "─";  // thin rule segment
const I_DOT:    &str = "·";  // faint separator dot

/// Spinner frames for reasoning animation
pub const SPINNER: [&str; 4] = [I_HALF, I_HALF2, I_HALF3, I_HALF4];

// ══════════════════════════════════════════════════════════════════════════════
// DIPRALIX Logo — Spectral Ghost Layer
// Uses ░ fill chars. Printed first, then cursor moves UP so the
// main colored art overlays on top (ghost peeks through edges).
// ══════════════════════════════════════════════════════════════════════════════

const DIPRALIX_GHOST: [&str; 6] = [
    " ░██████░  ░██░ ░██████░ ░██████░  ░█████░ ",
    " ░██░░░██░ ░██░ ░██░░░██░░██░░░██░░██░░░██░",
    " ░██░  ██░ ░██░ ░██████░ ░██████░ ░███████░",
    " ░██░  ██░ ░██░ ░██░░░░  ░██░░░██░░██░░░██░",
    " ░██████░  ░██░ ░██░     ░██░  ██░░██░  ██░",
    "                                           ",
];

// ══════════════════════════════════════════════════════════════════════════════
// DIPRALIX Logo — Colored Main Art
// Each letter colored: D=FIRE I=PLASMA P=BRIGHT R=VIOLET A=MINT
// ══════════════════════════════════════════════════════════════════════════════

type Segment = (&'static str, &'static str); // (ansi_color, text)

fn dipralix_lines() -> [[Segment; 5]; 6] {
    [
        [
            (FIRE,   "██████╗ "),
            (PLASMA, " ██╗ "),
            (BRIGHT, "██████╗ "),
            (VIOLET, "██████╗ "),
            (MINT,   " █████╗ "),
        ],
        [
            (FIRE,   "██╔══██╗"),
            (PLASMA, " ██║ "),
            (BRIGHT, "██╔══██╗"),
            (VIOLET, "██╔══██╗"),
            (MINT,   "██╔══██╗"),
        ],
        [
            (FIRE,   "██║  ██║"),
            (PLASMA, " ██║ "),
            (BRIGHT, "██████╔╝"),
            (VIOLET, "██████╔╝"),
            (MINT,   "███████║"),
        ],
        [
            (FIRE,   "██║  ██║"),
            (PLASMA, " ██║ "),
            (BRIGHT, "██╔═══╝ "),
            (VIOLET, "██╔══██╗"),
            (MINT,   "██╔══██║"),
        ],
        [
            (FIRE,   "██████╔╝"),
            (PLASMA, " ██║ "),
            (BRIGHT, "██║     "),
            (VIOLET, "██║  ██║"),
            (MINT,   "██║  ██║"),
        ],
        [
            (FIRE,   "╚═════╝ "),
            (PLASMA, " ╚═╝ "),
            (BRIGHT, "╚═╝     "),
            (VIOLET, "╚═╝  ╚═╝"),
            (MINT,   "╚═╝  ╚═╝"),
        ],
    ]
}

// ══════════════════════════════════════════════════════════════════════════════
// Logo renderer — ghost + overlay technique
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_dipralix_logo() {
    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    // ── 1. Print ghost (dim, 1-col offset to the right) ──────────────────────
    for line in &DIPRALIX_GHOST {
        writeln!(w, "{GHOST}{DIM}  {line}{RESET}").unwrap();
    }

    // ── 2. Move cursor UP to beginning of ghost ───────────────────────────────
    write!(w, "{}", cursor_up(DIPRALIX_GHOST.len())).unwrap();

    // ── 3. Overlay main colored art — ghost ░ chars peek around the edges ─────
    for segments in &dipralix_lines() {
        write!(w, "{CLRLN}{} ", cursor_col_1()).unwrap();
        for (color, text) in segments {
            write!(w, "{color}{text}").unwrap();
        }
        writeln!(w, "{RESET}").unwrap();
    }

    w.flush().unwrap();
}

// ══════════════════════════════════════════════════════════════════════════════
// Startup Banner — NULLVOID::PROTOCOL
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_banner() {
    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    // ── Protocol header ───────────────────────────────────────────────────────
    let bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "dipralix-cli".into());
    let pid    = std::process::id();
    let arch   = std::env::consts::ARCH;
    let os     = std::env::consts::OS;
    let target = format!("{}-unknown-{}-gnu", arch, os);

    writeln!(w).unwrap();
    writeln!(
        w,
        " {TEXT}NULLVOID::PROTOCOL {MUTED}# {TEXT}{bin} \
         {MUTED}{I_DOT} {TEXT}pid:{pid} \
         {MUTED}{I_DOT} {TEXT}{target}{RESET}"
    ).unwrap();
    thin_rule(&mut w);

    // ── DIPRALIX logo (ghost + overlay) ─────────────────────────────────────────
    w.flush().unwrap();
    drop(w);
    print_dipralix_logo();
    let stdout = io::stdout();
    let mut w = io::BufWriter::new(stdout.lock());

    // ── Status pills — pipe-separated, one line ───────────────────────────────
    writeln!(
        w,
        " {MINT}{I_MARK} Safety{MINT}:ON{RESET}  {MUTED}|{RESET} \
         {FIRE}{I_PROC} 14 Tools{RESET}  {MUTED}|{RESET} \
         {PLASMA}{I_GRID} 1M Context{RESET}  {MUTED}|{RESET} \
         {VIOLET}{I_TARGET} Multi-Model{RESET}  {MUTED}|{RESET} \
         {AMBER}{I_BRANCH} .dipralix/project.md{RESET}"
    ).unwrap();
    thin_rule(&mut w);

    // ── Command hints ─────────────────────────────────────────────────────────
    writeln!(
        w,
        " {MUTED}{I_PROMPT}  {TEXT}/help {MUTED}{I_DOT} {TEXT}/think {MUTED}{I_DOT} \
         {TEXT}/web {MUTED}{I_DOT} {TEXT}/model {MUTED}{I_DOT} {TEXT}/task {MUTED}{I_DOT} \
         {TEXT}/undo {MUTED}{I_DOT} {TEXT}/compact {MUTED}{I_DOT} {TEXT}/session {MUTED}{I_DOT} \
         {TEXT}/security {MUTED}{I_DOT} {TEXT}/quit{RESET}"
    ).unwrap();

    writeln!(w).unwrap();
    w.flush().unwrap();
}

// ══════════════════════════════════════════════════════════════════════════════
// Mode selector (Voice / Text)
// ══════════════════════════════════════════════════════════════════════════════

/// Returns `true` if user chose Text mode.
pub fn print_mode_selector() -> bool {
    println!(
        "\n {MINT}{I_LIVE}{RESET} {BRIGHT}Mic detected{RESET} \
         {MUTED}—{RESET} {TEXT}Voice mode or Text mode?{RESET}"
    );
    println!(
        " {MUTED}[{MINT}1{MUTED}] {BRIGHT}Voice (EMBER){RESET}  \
         {MUTED}|  [{MINT}2{MUTED}] {BRIGHT}Text (terminal){RESET}  \
         {MUTED}|  [Enter] = Voice{RESET}"
    );
    print!(" {PLASMA}{I_PROMPT}{RESET} ");
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim() == "2"
}

// ══════════════════════════════════════════════════════════════════════════════
// Model auto-detect notification
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_model_detect(model: &str, provider: &str, count: usize) {
    println!(
        " {I_WARN} Auto-detected: {AMBER}{model}{TEXT} ({provider}, {count} models){RESET}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Project loaded
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_project_loaded(path: &str) {
    println!(
        "\n {MINT}{I_ADD}{RESET} Loaded project instructions from \
         {AMBER}{path}{RESET}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Chat rendering
// ══════════════════════════════════════════════════════════════════════════════

/// The blinking cursor waiting for user input.
pub fn print_input_prompt() {
    print!("\n {PLASMA}{I_PROMPT}  {MUTED}╰─ {MINT}>{RESET} ");
    io::stdout().flush().unwrap();
}

/// Return the prompt string for use with readline / rustyline.
pub fn input_prompt_str() -> String {
    format!(" {PLASMA}{I_PROMPT}  {MUTED}╰─ {MINT}>{RESET} ")
}

/// Echo the user's typed message back in styled form.
pub fn print_user_echo(text: &str) {
    println!(
        "\n {MUTED}{I_PROMPT}  ╰─ {BRIGHT}>{RESET} {BRIGHT}{text}{RESET}"
    );
}

/// Reasoning header shown while model is thinking.
pub fn print_thinking(model: &str) {
    println!(
        "\n {VIOLET}{I_OUT} REASONING {MUTED}[{AMBER}{model}{MUTED}]{TEXT} \
         {I_STREAM}{I_STREAM}{I_STREAM}{RESET}"
    );
}

/// Animated spinner frame — call repeatedly while awaiting API response.
/// Pass the frame index (0–3) to cycle through `SPINNER`.
pub fn print_thinking_frame(frame: usize) {
    print!("\r {VIOLET}{}{RESET} ", SPINNER[frame % SPINNER.len()]);
    io::stdout().flush().unwrap();
}

/// Render a single streaming thought line inside the reasoning block.
pub fn print_thinking_line(line: &str) {
    println!(" {MUTED}│  {AMBER}{line}{RESET}");
}

/// Close the reasoning block after streaming thoughts.
pub fn print_thinking_close() {
    println!(" {MUTED}╰{}{RESET}", I_RULE.repeat(47));
}

/// Top border of a response block.
pub fn print_response_header() {
    println!();
    println!(" {MUTED}╭──  {PLASMA}{}{RESET}", I_RULE.repeat(44));
    println!(" {MUTED}│  {VIOLET}{I_OUT}{BRIGHT} OUTPUT{RESET}");
    println!(" {PLASMA}├──  {MUTED}{}{RESET}", I_RULE.repeat(44));
}

/// Render lines of AI response text inside the response block.
pub fn print_response_body(text: &str) {
    for line in text.lines() {
        if line.is_empty() {
            println!(" {MUTED}│{RESET}");
        } else {
            println!(" {MUTED}│  {TEXT}{line}{RESET}");
        }
    }
    println!(" {MUTED}╰{}{RESET}", I_RULE.repeat(47));
}

/// Token stats footer after a response.
pub fn print_token_stats(
    prompt_k:     u32,
    completion:   u32,
    thinking:     u32,
    session_usd:  f64,
    input_total:  u32,
    output_total: u32,
    turn:         u32,
) {
    println!(
        " {MUTED}{I_ACTIVE}  {TEXT}{prompt_k}Kp  {completion}c  {thinking}t  \
         {MUTED}{I_STREAM}  {TEXT}{}K tokens{RESET}",
        (prompt_k * 1000 + completion + thinking) / 1000
    );
    println!(
        " {MUTED}{I_WARN}  Session: {AMBER}${session_usd:.4}  \
         {MUTED}{I_STREAM}  {TEXT}Input: {input_total}K  \
         {MUTED}{I_STREAM}  {TEXT}Output: {output_total}K  \
         {MUTED}{I_STREAM}  {TEXT}Turns: {turn}{RESET}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Tool-use notifications
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_tool_call(tool: &str, args_summary: &str) {
    println!(
        " {AMBER}{I_PROC}{RESET}  {BRIGHT}{tool}{RESET}  {MUTED}{args_summary}{RESET}"
    );
}

pub fn print_tool_result(success: bool, summary: &str) {
    let (icon, color) = if success { (I_ADD, MINT) } else { (I_ERROR, FIRE) };
    println!(" {color}{icon}{RESET}  {TEXT}{summary}{RESET}");
}

// ══════════════════════════════════════════════════════════════════════════════
// Error / warning / info display
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_error(msg: &str) {
    println!("\n {FIRE}{I_ERROR} ERROR{RESET}  {TEXT}{msg}{RESET}");
}

pub fn print_warning(msg: &str) {
    println!(" {AMBER}{I_WARN}  {TEXT}{msg}{RESET}");
}

pub fn print_info(msg: &str) {
    println!(" {PLASMA}{I_MARK}  {TEXT}{msg}{RESET}");
}

// ══════════════════════════════════════════════════════════════════════════════
// Compact session view  /compact
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_session_summary(
    turns:     u32,
    total_in:  u32,
    total_out: u32,
    cost_usd:  f64,
) {
    println!();
    thin_rule_stdout();
    println!(" {VIOLET}{I_MARK}  SESSION SUMMARY{RESET}");
    println!(" {MUTED}{I_STREAM}  Turns:   {BRIGHT}{turns}{RESET}");
    println!(" {MUTED}{I_STREAM}  Input:   {BRIGHT}{total_in}K tokens{RESET}");
    println!(" {MUTED}{I_STREAM}  Output:  {BRIGHT}{total_out}K tokens{RESET}");
    println!(" {MUTED}{I_STREAM}  Cost:    {AMBER}${cost_usd:.4}{RESET}");
    thin_rule_stdout();
    println!();
}

// ══════════════════════════════════════════════════════════════════════════════
// Security mode banner  /security
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_security_status(safety_on: bool) {
    let (icon, color, label) = if safety_on {
        (I_LIVE, MINT, "ENABLED")
    } else {
        (I_ERROR, FIRE, "DISABLED — CAUTION")
    };
    println!("\n {color}{icon}  Safety Policy: {BRIGHT}{label}{RESET}");
}

// ══════════════════════════════════════════════════════════════════════════════
// Quit message
// ══════════════════════════════════════════════════════════════════════════════

pub fn print_quit(session_usd: f64) {
    println!();
    thin_rule_stdout();
    println!(
        " {MUTED}{I_MARK}  {TEXT}Session closed  \
         {MUTED}{I_STREAM}  {AMBER}Total: ${session_usd:.4}{RESET}"
    );
    thin_rule_stdout();
    println!();
}

// ══════════════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════════════

fn thin_rule(w: &mut impl Write) {
    writeln!(w, " {MUTED}{}{RESET}", I_RULE.repeat(68)).unwrap();
}

pub fn thin_rule_stdout() {
    println!(" {MUTED}{}{RESET}", I_RULE.repeat(68));
}

// ══════════════════════════════════════════════════════════════════════════════
// Demo main — reproduces the session shown in the original paste
// ══════════════════════════════════════════════════════════════════════════════

fn main() {
    // Clear terminal
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();

    // ── Boot ─────────────────────────────────────────────────────────────────
    println!();
    print_model_detect("gemini-3-flash-preview", "Gemini", 50);
    println!();
    print_banner();

    // ── Mode selector ─────────────────────────────────────────────────────────
    println!(
        " {MINT}{I_LIVE}{RESET} {BRIGHT}Mic detected{RESET} \
         {MUTED}—{RESET} {TEXT}Voice mode or Text mode?{RESET}"
    );
    println!(
        " {MUTED}[{MINT}1{MUTED}] {BRIGHT}Voice (EMBER){RESET}  \
         {MUTED}|  [{MINT}2{MUTED}] {BRIGHT}Text (terminal){RESET}  \
         {MUTED}|  [Enter] = Voice{RESET}"
    );
    println!(" {PLASMA}{I_PROMPT}{RESET}  {TEXT}2{RESET}");

    print_project_loaded(".forge/project.md");

    // ── Turn 1: hey ───────────────────────────────────────────────────────────
    print_input_prompt();
    println!("{TEXT}hey{RESET}");
    print_user_echo("hey");
    print_thinking("gemini-3-flash");
    print_response_header();
    print_response_body(
        "Hey. I'm DIPRALIX. I'm ready to work on the codebase.\n\
         \n\
         I've loaded the project context:\n\
         — Project: DIPRALIX (Rust-based AI coding agent)\n\
         — Tech Stack: Rust, Tokio, Reqwest, Clap, Serde\n\
         — Working Directory: /home/pratik/Desktop/dipralix\n\
         \n\
         What are we building today?"
    );
    print_token_stats(3, 103, 52, 0.0006, 3, 0, 1);

    // ── Turn 2: how are you ───────────────────────────────────────────────────
    print_input_prompt();
    println!("{TEXT}how are you{RESET}");
    print_user_echo("how are you");
    print_thinking("gemini-3-flash");
    print_response_header();
    print_response_body(
        "Operating at peak performance.\n\
         Context window clear, tools primed, ready to ship.\n\
         \n\
         What's on the DIPRALIX roadmap today? Ready to dive into\n\
         the Rust source, run tests, or implement new features."
    );
    print_token_stats(3, 68, 96, 0.0013, 7, 0, 2);

    // ── Turn 3: am i horrible ────────────────────────────────────────────────
    print_input_prompt();
    println!("{TEXT}am i horrible{RESET}");
    print_user_echo("am i horrible");
    print_thinking("gemini-3-flash");
    print_response_header();
    print_response_body(
        "Definitely not. You're the one steering this project,\n\
         and I'm here to handle the heavy lifting.\n\
         \n\
         If there's a frustrating bug or block, let's look at it.\n\
         What's the next task on the list?"
    );
    print_token_stats(3, 92, 179, 0.0019, 11, 0, 3);

    // ── Awaiting next input ───────────────────────────────────────────────────
    print_input_prompt();
    io::stdout().flush().unwrap();

    // In production: feed into your readline / rustyline event loop here.
    // e.g.  let mut task = String::new();
    //       io::stdin().read_line(&mut task).unwrap();
    //       process_task(task.trim());
}