#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;
extern crate napi_sys;

use std::convert::TryInto;

use napi::{
  Env, CallContext,Property,
  JsNumber, JsObject, JsString, JsBuffer, JsBoolean, JsUndefined, JsUnknown,
  Result, Status,
  NapiRaw,
  Task,
};

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

#[js_function(1)]
fn info(ctx: CallContext) -> Result<JsNumber> {
  let jsb: JsBuffer = ctx.get::<JsBuffer>(0)?;

  if !jsb.is_buffer()? {
    let e = napi::Error {status: Status::InvalidArg, reason: "expected a buffer".to_string()};
    unsafe {
      napi::JsTypeError::from(e).throw_into(ctx.env.raw());
    }
  }

  let l: u32 = jsb.get_named_property::<JsNumber>("length")?.try_into()?;
  // into_value() conveniently returns &mut [u8]
  let buf = &mut jsb.into_value()?;

  ctx.env.create_uint32(buf[1] as u32)
}

//
// this version needs napi_sys
//
#[js_function(1)]
fn info2(ctx: CallContext) -> Result<JsNumber> {
  let mut len: usize = 0;
  let mut ptr = std::ptr::null_mut();

  let jsb: JsBuffer = ctx.get::<JsBuffer>(0)?;

  if !jsb.is_buffer()? {
    let e = napi::Error {status: Status::InvalidArg, reason: "expected a buffer".to_string()};
    unsafe {
      napi::JsTypeError::from(e).throw_into(ctx.env.raw());
    }
  }

  unsafe {
    //use napi::NapiRaw;
    let status: napi_sys::napi_status = napi_sys::napi_get_buffer_info(
      ctx.env.raw(),
      jsb.raw(),
      &mut ptr,
      &mut len
    );
    if status != napi_sys::Status::napi_ok {
      return ctx.env.create_int32(-status);
    }
  }
  ctx.env.create_uint32(len as u32)
}

#[js_function(1)]
fn info3(ctx: CallContext) -> Result<JsBoolean> {
    let mut ptr: napi_value = std::ptr::null_mut();
    let mut val: napi_value = std::ptr::null_mut();

    let jsu: JsUnknown = ctx.get::<JsUnknown>(0)?;

    unsafe {
        //use napi::NapiRaw;
        let mut status: napi_sys::napi_status = napi_sys::napi_coerce_to_bool(
            ctx.env.raw(),
            jsu.raw(),
            &mut val
        );
        if status != napi_sys::Status::napi_ok {
            let e = napi::Error {status: Status::InvalidArg, reason: "hmmm".to_string()};
            napi::JsTypeError::from(e).throw_into(ctx.env.raw());
        }
        let mut b: &mut bool = &mut false;
        status = napi_sys::napi_get_value_bool(ctx.env.raw(), val, &mut *b);
        if status != napi_sys::Status::napi_ok {
            let e = napi::Error {status: Status::InvalidArg, reason: "hmmm2".to_string()};
            napi::JsTypeError::from(e).throw_into(ctx.env.raw());
        }

        ctx.env.get_boolean(*b)
    }
}

//+++++++++++++++++++++++++++++++++
// class-based approach
//---------------------------------
struct Scanner {
  bad_chars: [bool; 256],
  prev_byte: u8
}

const DASH: u8 = '-' as u8;

#[js_function(1)]
fn scanner_constructor(ctx: CallContext) -> Result<JsUndefined> {
  let mut bad_chars: [bool; 256] = [false; 256];

  let stop_chars = &mut ctx.get::<JsBuffer>(0)?.into_value()?;

  for stop_char in stop_chars.into_iter() {
    bad_chars[*stop_char as usize] = true;
  }

  let mut scanner = Scanner {bad_chars, prev_byte: 0xFF};

  let mut this: JsObject = ctx.this_unchecked();
  ctx.env.wrap(&mut this, scanner)?;

  ctx.env.get_undefined()
}

#[js_function(1)]
fn scanner_get(ctx: CallContext) -> Result<JsBoolean> {
  let ix: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
  let this: JsObject = ctx.this_unchecked();
  let scanner: &mut Scanner = ctx.env.unwrap(&this)?;

  ctx.env.get_boolean(scanner.bad_chars[ix as usize])
}

#[js_function(1)]
fn scanner_suspicious(ctx: CallContext) -> Result<JsBoolean> {
  let bytes = &mut ctx.get::<JsBuffer>(0)?.into_value()?;
  let this: JsObject = ctx.this_unchecked();
  let scanner: &mut Scanner = ctx.env.unwrap(&this)?;

  for byte in bytes.into_iter() {
    if scanner.bad_chars[*byte as usize] {
      return ctx.env.get_boolean(true);
    }
    if *byte == DASH && scanner.prev_byte == DASH {
      return ctx.env.get_boolean(true);
    }
    scanner.prev_byte = *byte;
  }
  ctx.env.get_boolean(false)
}


//
// not ~~very easy~~ possible to return multiple types with this particular api
//
#[js_function(1)]
fn init_bad_chars(ctx: CallContext) -> Result<JsNumber> {
  let jss = ctx.get::<JsString>(0)?;
  let chars = jss.into_utf8()?;
  //let chars = ctx.get::<JsString>(0)?.into_utf8()?;
  // iterate over characters
  //let text = format!("{} world!", chars.as_str()?);
  //ctx.env.create_string(text.as_str())
  let l: usize = jss.utf8_len()?;
  ctx.env.create_uint32(l as u32)
}

const INIT_ARG_COUNT: usize = 1;
//
// fall back to napi_ for more flexibility
//
use napi_sys::{
  napi_env, napi_callback_info, napi_get_cb_info,
  napi_status,
  napi_value,
};
extern "C" fn napi_init(env: napi_env, info: napi_callback_info) -> napi_value {
  let mut argc: usize = INIT_ARG_COUNT;

  let mut argv: [napi_value; INIT_ARG_COUNT] = [std::ptr::null_mut(); INIT_ARG_COUNT];
  let mut this_arg: napi_value = std::ptr::null_mut();

  unsafe {
    let status: napi_status = napi_get_cb_info(env, info, &mut argc, argv.as_mut_ptr(), &mut this_arg, std::ptr::null_mut());
  }

  let mut result: napi_value = std::ptr::null_mut();

  // thank you reddit
  // https://www.reddit.com/r/rust/comments/96om71/how_to_allocate_and_pass_byte_array_to_c_function/

  let slice = unsafe {std::slice::from_raw_parts(argv.as_mut_ptr(), argc)};

  slice[0]
}

struct Test {
  value: i32
}

impl Test {
  pub fn new(value: i32) -> std::result::Result<Self, String> {
    if value > 42 {
      return Err(String::from("error"));
    }
    Ok(Test {value})
  }
}


//
// exports
//
#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  exports.create_named_method("sync", sync_fn)?;
  exports.create_named_method("sleep", sleep)?;
  exports.create_named_method("info", info)?;
  exports.create_named_method("info2", info2)?;
  exports.create_named_method("info3", info3)?;
  exports.create_named_method("setStopChars", init_bad_chars)?;
  exports.create_named_method("init", napi_init)?;

  let sclass = env.define_class("Scanner", scanner_constructor, &[
    Property::new(&env, "get")?.with_method(scanner_get),
    Property::new(&env, "suspicious")?.with_method(scanner_suspicious),
  ])?;
  exports.set_named_property("Scanner", sclass)?;
  Ok(())
}
