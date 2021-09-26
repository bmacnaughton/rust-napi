#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::convert::TryInto;

use napi::{CallContext, Env, JsNumber, JsObject, Result, Task, Status, JsBuffer};

#[cfg(all(
  any(windows, unix),
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct AsyncTask(u32);

impl Task for AsyncTask {
  type Output = u32;
  type JsValue = JsNumber;

  fn compute(&mut self) -> Result<Self::Output> {
    use std::thread::sleep;
    use std::time::Duration;
    sleep(Duration::from_millis(self.0 as u64));
    Ok(self.0 * 2)
  }

  fn resolve(self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
    env.create_uint32(output)
  }
}

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("sync", sync_fn)?;
  exports.create_named_method("sleep", sleep)?;
  exports.create_named_method("info", info)?;
  Ok(())
}

#[js_function(1)]
fn sync_fn(ctx: CallContext) -> Result<JsNumber> {
  let argument: u32 = ctx.get::<JsNumber>(0)?.try_into()?;

  ctx.env.create_uint32(argument + 100)
}

#[js_function(1)]
fn sleep(ctx: CallContext) -> Result<JsObject> {
  let argument: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
  let task = AsyncTask(argument);
  let async_task = ctx.env.spawn(task)?;
  Ok(async_task.promise_object())
}

//
//
//
extern "C" {
  fn napi_get_buffer_info(
    env: &Env,
    value: JsBuffer,
    data: *mut *mut u8,
    length: *mut usize
  ) -> Status;
}

#[js_function(1)]
fn info(ctx: CallContext) -> Result<JsNumber> {
  //let mut len: usize = 0;
  //let mut ptr: *mut u8 = std::ptr::null_mut();
  let jsb: JsBuffer = ctx.get::<JsBuffer>(0)?;

  //const is_buf: Result = jsb.is_buffer();
  //match is_buf {
  //
  //}
  if !jsb.is_buffer()? {
    let e = napi::Error {status: Status::InvalidArg, reason: "expected a buffer".to_string()};
    unsafe {
      napi::JsTypeError::from(e).throw_into(ctx.env.raw());
    }
  }

  let l: u32 = jsb.get_named_property::<JsNumber>("length")?.try_into()?;
  let buf = &mut jsb.into_value()?; // &mut [u8]

  ctx.env.create_uint32(buf[1] as u32)


  //unsafe {
  //  let status: Status = napi_get_buffer_info(ctx.env, b, &mut ptr, &mut len);
  //  if status != Status::Ok {
  //    return ctx.env.create_uint32(9999);
  //  }
  //}
  //ctx.env.create_uint32(len as u32)
}
