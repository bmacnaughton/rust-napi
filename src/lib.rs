#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;
extern crate napi_sys;

use std::convert::TryInto;

use napi::{
  Env, CallContext,
  JsNumber, JsObject, JsUndefined,
  Result,
};

#[cfg(all(
  any(windows, unix),
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct Mem<> {
    _buffer: Vec<u64>,
    _n: usize,
}

#[js_function(1)]
fn mem_constructor(ctx: CallContext) -> Result<JsUndefined> {
    let n: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
    let n = n as usize;
    let buffer = Vec::<u64>::with_capacity(n);
    let mem = Mem{_buffer: buffer, _n: n};

    let mut this: JsObject = ctx.this_unchecked();
    ctx.env.wrap(&mut this, mem)?;

    ctx.env.get_undefined()
}

//
// exports
//
#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  let mclass = env.define_class("Mem", mem_constructor, &[])?;
  exports.set_named_property("Mem", mclass)?;
  Ok(())
}
