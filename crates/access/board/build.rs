//! Generates the messages of KiCad's IPC API from KiCad's own definitions in
//! `proto/kicad`, with a protobuf compiler written in Rust, so that building
//! traxide needs no `protoc`.

use std::io::Result;

const PROTOS: &str = "proto/kicad";

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed={PROTOS}");
    let definitions = protox::compile(
        [
            "common/envelope.proto",
            "common/commands/editor_commands.proto",
            "board/board_commands.proto",
        ],
        [PROTOS],
    )
    .map_err(std::io::Error::other)?;
    prost_build::Config::new()
        // KiCad's comments stay in its files: they are no contract of traxide's,
        // and rustdoc would take their indented lines for doctests.
        .disable_comments(["."])
        .enable_type_names()
        .type_name_domain(["."], "type.googleapis.com")
        .include_file("kiapi.rs")
        .compile_fds(definitions)
}
