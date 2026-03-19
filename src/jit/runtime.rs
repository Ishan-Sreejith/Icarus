
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::rc::Rc;

use crate::training_data;

pub type EncodedValue = u64;

const TAG_MASK: u64 = 1;
const TAG_INT: u64 = 1;

#[derive(Debug)]
pub enum GcData {
    String(String),
    List(Vec<EncodedValue>),
    Map(HashMap<String, EncodedValue>),
    Float(f64),
    File(Option<File>), // Option to allow closing
}

type GcPtr = *const RefCell<GcData>;


#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct ExceptionHandler {
    pub sp: u64,
    pub fp: u64,
    pub lr: u64, // The address of the catch block
}

thread_local! {
    static EXCEPTION_STACK: RefCell<Vec<ExceptionHandler>> = RefCell::new(Vec::new());
    static LAST_ERROR: RefCell<EncodedValue> = RefCell::new(0);
}

fn is_int(val: EncodedValue) -> bool {
    (val & TAG_MASK) == TAG_INT
}

fn decode_int(val: EncodedValue) -> i64 {
    (val as i64) >> 1
}

fn encode_int(val: i64) -> EncodedValue {
    ((val as u64) << 1) | TAG_INT
}

#[allow(dead_code)]
fn ptr_to_rc(ptr: EncodedValue) -> Rc<RefCell<GcData>> {
    unsafe { Rc::from_raw(ptr as GcPtr) }
}

fn rc_to_ptr(rc: Rc<RefCell<GcData>>) -> EncodedValue {
    Rc::into_raw(rc) as EncodedValue
}

fn get_string(val: EncodedValue) -> String {
    if is_int(val) {
        decode_int(val).to_string()
    } else {
        if val == 0 {
            return "null".to_string();
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let s = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::String(s) => s.clone(),
                GcData::Float(f) => f.to_string(),
                _ => format!("{:?}", borrow),
            }
        };
        let _ = Rc::into_raw(rc);
        s
    }
}

fn format_value(val: EncodedValue) -> String {
    if is_int(val) {
        decode_int(val).to_string()
    } else {
        if val == 0 {
            return "null".to_string();
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let s = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::String(s) => format!("\"{}\"", s),
                GcData::Float(f) => f.to_string(),
                GcData::List(l) => {
                    let items: Vec<String> = l.iter().map(|&item| format_value(item)).collect();
                    format!("[{}]", items.join(", "))
                }
                GcData::Map(m) => {
                    let pairs: Vec<String> = m.iter()
                        .map(|(k, &v)| format!("\"{}\": {}", k, format_value(v)))
                        .collect();
                    format!("{{{}}}", pairs.join(", "))
                }
                GcData::File(_) => "<File>".to_string(),
            }
        };
        let _ = Rc::into_raw(rc);
        s
    }
}

#[no_mangle]
pub extern "C" fn rt_is_map(val: EncodedValue) -> EncodedValue {
    if is_int(val) || val == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(val as GcPtr) };
    let is_map = matches!(&*rc.borrow(), GcData::Map(_));
    let _ = Rc::into_raw(rc);
    encode_int(if is_map { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_is_list(val: EncodedValue) -> EncodedValue {
    if is_int(val) || val == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(val as GcPtr) };
    let is_list = matches!(&*rc.borrow(), GcData::List(_));
    let _ = Rc::into_raw(rc);
    encode_int(if is_list { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_is_string(val: EncodedValue) -> EncodedValue {
    if is_int(val) || val == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(val as GcPtr) };
    let is_string = matches!(&*rc.borrow(), GcData::String(_));
    let _ = Rc::into_raw(rc);
    encode_int(if is_string { 1 } else { 0 })
}


#[no_mangle]
pub extern "C" fn rt_retain(val: EncodedValue) {
    if is_int(val) || val == 0 {
        return;
    }
    unsafe {
        Rc::increment_strong_count(val as GcPtr);
    }
}

#[no_mangle]
pub extern "C" fn rt_release(val: EncodedValue) {
    if is_int(val) || val == 0 {
        return;
    }
    unsafe {
        Rc::decrement_strong_count(val as GcPtr);
    }
}


#[no_mangle]
pub extern "C" fn rt_alloc_string(buffer: *const u8, len: u64) -> EncodedValue {
    unsafe {
        let slice = std::slice::from_raw_parts(buffer, len as usize);
        let s = String::from_utf8_lossy(slice).into_owned();
        let rc = Rc::new(RefCell::new(GcData::String(s)));
        rc_to_ptr(rc)
    }
}

#[no_mangle]
pub extern "C" fn rt_input(prompt: EncodedValue) -> EncodedValue {
    let prompt_str = get_string(prompt);

    if let Some(answer) = training_data::lookup(&prompt_str) {
        let rc = Rc::new(RefCell::new(GcData::String(answer)));
        return rc_to_ptr(rc);
    }

    print!("{}", prompt_str);
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return 0;
    }

    let input = input.trim().to_string();
    let rc = Rc::new(RefCell::new(GcData::String(input)));
    rc_to_ptr(rc)
}

#[no_mangle]
pub extern "C" fn rt_alloc_list(capacity: u64) -> EncodedValue {
    let vec = Vec::with_capacity(capacity as usize);
    let rc = Rc::new(RefCell::new(GcData::List(vec)));
    rc_to_ptr(rc)
}

#[no_mangle]
pub extern "C" fn rt_alloc_map() -> EncodedValue {
    let map = HashMap::new();
    let rc = Rc::new(RefCell::new(GcData::Map(map)));
    rc_to_ptr(rc)
}

#[no_mangle]
pub extern "C" fn rt_alloc_float(val: f64) -> EncodedValue {
    let rc = Rc::new(RefCell::new(GcData::Float(val)));
    rc_to_ptr(rc)
}


#[no_mangle]
pub extern "C" fn rt_add(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if !is_int(a) || !is_int(b) {
        let s = get_string(a) + &get_string(b);
        let rc = Rc::new(RefCell::new(GcData::String(s)));
        return rc_to_ptr(rc);
    }
    encode_int(decode_int(a) + decode_int(b))
}

#[no_mangle]
pub extern "C" fn rt_sub(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if !is_int(a) || !is_int(b) {
        return encode_int(get_float(a) as i64 - get_float(b) as i64);
    }
    encode_int(decode_int(a) - decode_int(b))
}

#[no_mangle]
pub extern "C" fn rt_mul(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if !is_int(a) || !is_int(b) {
        return rt_alloc_float(get_float(a) * get_float(b));
    }
    encode_int(decode_int(a) * decode_int(b))
}

#[no_mangle]
pub extern "C" fn rt_div(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if !is_int(a) || !is_int(b) {
        return rt_float_div(a, b);
    }
    let denom = decode_int(b);
    if denom == 0 {
        return encode_int(0);
    }
    encode_int(decode_int(a) / denom)
}

#[no_mangle]
pub extern "C" fn rt_eq(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if is_int(a) && is_int(b) {
        if std::env::var("CORE_JIT_TRACE").is_ok() {
            eprintln!("[rt_eq] a={} b={}", decode_int(a), decode_int(b));
        }
        return encode_int(if decode_int(a) == decode_int(b) { 1 } else { 0 });
    }
    encode_int(if get_string(a) == get_string(b) { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_lt(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if is_int(a) && is_int(b) {
        return encode_int(if decode_int(a) < decode_int(b) { 1 } else { 0 });
    }
    encode_int(if get_float(a) < get_float(b) { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_gt(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if is_int(a) && is_int(b) {
        return encode_int(if decode_int(a) > decode_int(b) { 1 } else { 0 });
    }
    encode_int(if get_float(a) > get_float(b) { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_is_truthy(val: EncodedValue) -> EncodedValue {
    if std::env::var("CORE_JIT_TRACE").is_ok() {
        eprintln!("[rt_is_truthy] val={}", get_string(val));
    }
    if val == 0 {
        return encode_int(0);
    }
    if is_int(val) {
        return encode_int(if decode_int(val) != 0 { 1 } else { 0 });
    }
    encode_int(1)
}

#[no_mangle]
pub extern "C" fn rt_not(val: EncodedValue) -> EncodedValue {
    if std::env::var("CORE_JIT_TRACE").is_ok() {
        eprintln!("[rt_not] val={}", get_string(val));
    }
    let t = rt_is_truthy(val);
    encode_int(if decode_int(t) == 0 { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_and(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    let ta = rt_is_truthy(a);
    if decode_int(ta) == 0 {
        return encode_int(0);
    }
    let tb = rt_is_truthy(b);
    encode_int(if decode_int(tb) != 0 { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_or(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    let ta = rt_is_truthy(a);
    if decode_int(ta) != 0 {
        return encode_int(1);
    }
    let tb = rt_is_truthy(b);
    encode_int(if decode_int(tb) != 0 { 1 } else { 0 })
}

#[no_mangle]
pub extern "C" fn rt_ne(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    let eq = rt_eq(a, b);
    encode_int(if decode_int(eq) == 0 { 1 } else { 0 })
}


#[no_mangle]
pub extern "C" fn rt_print(val: EncodedValue) {
    if is_int(val) {
        println!("{}", decode_int(val));
    } else {
        if val == 0 {
            println!("null");
            return;
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        match rc.try_borrow() {
            Ok(borrow) => match &*borrow {
                GcData::String(s) => println!("{}", s),
                GcData::List(l) => {
                    print!("[");
                    for (i, item) in l.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("{}", format_value(*item));
                    }
                    println!("]");
                }
                GcData::Map(m) => {
                    print!("{{");
                    let mut first = true;
                    for (key, value) in m {
                        if !first {
                            print!(", ");
                        }
                        first = false;
                        print!("\"{}\": {}", key, format_value(*value));
                    }
                    println!("}}");
                }
                GcData::Float(f) => println!("{}", f),
                GcData::File(_) => println!("<File>"),
            },
            Err(_) => println!("<Value borrowed>"),
        }
        let _ = Rc::into_raw(rc);
    }
}

#[no_mangle]
pub extern "C" fn rt_to_str(val: EncodedValue) -> EncodedValue {
    if is_int(val) {
        let s = decode_int(val).to_string();
        let rc = Rc::new(RefCell::new(GcData::String(s)));
        rc_to_ptr(rc)
    } else {
        if val == 0 {
            return rt_alloc_string("null".as_ptr(), 4);
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let s = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::String(s) => s.clone(),
                GcData::Float(f) => f.to_string(),
                _ => format!("{:?}", borrow),
            }
        };
        let _ = Rc::into_raw(rc);
        let new_rc = Rc::new(RefCell::new(GcData::String(s)));
        rc_to_ptr(new_rc)
    }
}

#[no_mangle]
pub extern "C" fn rt_to_num(val: EncodedValue) -> EncodedValue {
    if is_int(val) {
        val
    } else {
        if val == 0 {
            return encode_int(0);
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let res = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::String(s) => s.parse::<i64>().unwrap_or(0),
                GcData::Float(f) => *f as i64,
                _ => 0,
            }
        };
        let _ = Rc::into_raw(rc);
        encode_int(res)
    }
}


fn get_float(val: EncodedValue) -> f64 {
    if is_int(val) {
        decode_int(val) as f64
    } else {
        if val == 0 {
            return 0.0;
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let f = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::Float(f) => *f,
                GcData::String(s) => s.parse::<f64>().unwrap_or(0.0),
                _ => 0.0,
            }
        };
        let _ = Rc::into_raw(rc);
        f
    }
}

#[no_mangle]
pub extern "C" fn rt_float_add(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    rt_alloc_float(get_float(a) + get_float(b))
}

#[no_mangle]
pub extern "C" fn rt_float_sub(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    rt_alloc_float(get_float(a) - get_float(b))
}

#[no_mangle]
pub extern "C" fn rt_float_mul(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    rt_alloc_float(get_float(a) * get_float(b))
}

#[no_mangle]
pub extern "C" fn rt_float_div(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    let fb = get_float(b);
    if fb == 0.0 {
        return rt_alloc_float(0.0);
    } // Handle div by zero
    rt_alloc_float(get_float(a) / fb)
}

#[no_mangle]
pub extern "C" fn rt_abs(v: EncodedValue) -> EncodedValue {
    if is_int(v) {
        return encode_int(decode_int(v).abs());
    }
    rt_alloc_float(get_float(v).abs())
}

#[no_mangle]
pub extern "C" fn rt_min(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if is_int(a) && is_int(b) {
        return encode_int(decode_int(a).min(decode_int(b)));
    }
    rt_alloc_float(get_float(a).min(get_float(b)))
}

#[no_mangle]
pub extern "C" fn rt_max(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    if is_int(a) && is_int(b) {
        return encode_int(decode_int(a).max(decode_int(b)));
    }
    rt_alloc_float(get_float(a).max(get_float(b)))
}

#[no_mangle]
pub extern "C" fn rt_sqrt(v: EncodedValue) -> EncodedValue {
    rt_alloc_float(get_float(v).sqrt())
}

#[no_mangle]
pub extern "C" fn rt_pow(a: EncodedValue, b: EncodedValue) -> EncodedValue {
    rt_alloc_float(get_float(a).powf(get_float(b)))
}

#[no_mangle]
pub extern "C" fn rt_contains(haystack: EncodedValue, needle: EncodedValue) -> EncodedValue {
    let h = get_string(haystack);
    let n = get_string(needle);
    encode_int(if h.contains(&n) { 1 } else { 0 })
}


#[no_mangle]
pub extern "C" fn rt_list_push(list_ptr: EncodedValue, item: EncodedValue) {
    if is_int(list_ptr) || list_ptr == 0 {
        return;
    }
    rt_retain(item);
    let rc = unsafe { Rc::from_raw(list_ptr as GcPtr) };
    {
        let mut borrow = rc.borrow_mut();
        if let GcData::List(list) = &mut *borrow {
            list.push(item);
        }
    }
    let _ = Rc::into_raw(rc);
}

#[no_mangle]
pub extern "C" fn rt_list_get(list_ptr: EncodedValue, index: EncodedValue) -> EncodedValue {
    if is_int(list_ptr) || list_ptr == 0 {
        return encode_int(0);
    }
    let idx = if is_int(index) { decode_int(index) } else { 0 };
    if idx < 0 {
        return encode_int(0);
    }

    let rc = unsafe { Rc::from_raw(list_ptr as GcPtr) };
    let res = {
        let borrow = rc.borrow();
        if let GcData::List(list) = &*borrow {
            if (idx as usize) < list.len() {
                let item = list[idx as usize];
                rt_retain(item);
                item
            } else {
                encode_int(0)
            }
        } else {
            encode_int(0)
        }
    };
    let _ = Rc::into_raw(rc);
    res
}

#[no_mangle]
pub extern "C" fn rt_list_set(list_ptr: EncodedValue, index: EncodedValue, value: EncodedValue) {
    if is_int(list_ptr) || list_ptr == 0 {
        return;
    }
    let idx = if is_int(index) { decode_int(index) } else { 0 };
    if idx < 0 {
        return;
    }

    rt_retain(value);
    let rc = unsafe { Rc::from_raw(list_ptr as GcPtr) };
    {
        let mut borrow = rc.borrow_mut();
        if let GcData::List(list) = &mut *borrow {
            if (idx as usize) < list.len() {
                let old = list[idx as usize];
                rt_release(old);
                list[idx as usize] = value;
            }
        }
    }
    let _ = Rc::into_raw(rc);
}

#[no_mangle]
pub extern "C" fn rt_list_len(list_ptr: EncodedValue) -> EncodedValue {
    if is_int(list_ptr) || list_ptr == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(list_ptr as GcPtr) };
    let len = {
        let borrow = rc.borrow();
        match &*borrow {
            GcData::List(l) => l.len() as i64,
            GcData::String(s) => s.len() as i64,
            GcData::Map(m) => m.len() as i64,
            _ => 0,
        }
    };
    let _ = Rc::into_raw(rc);
    encode_int(len)
}

#[no_mangle]
pub extern "C" fn rt_list_pop(list_ptr: EncodedValue) -> EncodedValue {
    if is_int(list_ptr) || list_ptr == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(list_ptr as GcPtr) };
    let res = {
        let mut borrow = rc.borrow_mut();
        if let GcData::List(list) = &mut *borrow {
            list.pop().unwrap_or_else(|| encode_int(0))
        } else {
            encode_int(0)
        }
    };
    let _ = Rc::into_raw(rc);
    res
}


fn val_to_key(val: EncodedValue) -> String {
    if is_int(val) {
        decode_int(val).to_string()
    } else {
        if val == 0 {
            return "null".to_string();
        }
        let rc = unsafe { Rc::from_raw(val as GcPtr) };
        let s = {
            let borrow = rc.borrow();
            match &*borrow {
                GcData::String(s) => s.clone(),
                _ => format!("{:?}", borrow),
            }
        };
        let _ = Rc::into_raw(rc);
        s
    }
}

#[no_mangle]
pub extern "C" fn rt_map_set(map_ptr: EncodedValue, key: EncodedValue, value: EncodedValue) {
    if is_int(map_ptr) || map_ptr == 0 {
        return;
    }
    rt_retain(value);

    let k = val_to_key(key);
    let rc = unsafe { Rc::from_raw(map_ptr as GcPtr) };
    {
        let mut borrow = rc.borrow_mut();
        if let GcData::Map(map) = &mut *borrow {
            if let Some(old_val) = map.insert(k, value) {
                rt_release(old_val);
            }
        }
    }
    let _ = Rc::into_raw(rc);
}

#[no_mangle]
pub extern "C" fn rt_map_get(map_ptr: EncodedValue, key: EncodedValue) -> EncodedValue {
    if is_int(map_ptr) || map_ptr == 0 {
        return encode_int(0);
    }
    let k = val_to_key(key);

    let rc = unsafe { Rc::from_raw(map_ptr as GcPtr) };
    let res = {
        let borrow = rc.borrow();
        if let GcData::Map(map) = &*borrow {
            if let Some(&val) = map.get(&k) {
                rt_retain(val);
                val
            } else {
                encode_int(0)
            }
        } else {
            encode_int(0)
        }
    };
    let _ = Rc::into_raw(rc);
    res
}

#[no_mangle]
pub extern "C" fn rt_map_keys(map_ptr: EncodedValue) -> EncodedValue {
    if is_int(map_ptr) || map_ptr == 0 {
        return rt_alloc_list(0);
    }

    let rc = unsafe { Rc::from_raw(map_ptr as GcPtr) };
    let list_rc = {
        let borrow = rc.borrow();
        if let GcData::Map(map) = &*borrow {
            let mut keys = Vec::new();
            for k in map.keys() {
                let s_rc = Rc::new(RefCell::new(GcData::String(k.clone())));
                keys.push(rc_to_ptr(s_rc));
            }
            Rc::new(RefCell::new(GcData::List(keys)))
        } else {
            Rc::new(RefCell::new(GcData::List(vec![])))
        }
    };
    let _ = Rc::into_raw(rc);
    rc_to_ptr(list_rc)
}

#[no_mangle]
pub extern "C" fn rt_map_values(map_ptr: EncodedValue) -> EncodedValue {
    if is_int(map_ptr) || map_ptr == 0 {
        return rt_alloc_list(0);
    }

    let rc = unsafe { Rc::from_raw(map_ptr as GcPtr) };
    let list_rc = {
        let borrow = rc.borrow();
        if let GcData::Map(map) = &*borrow {
            let mut vals = Vec::with_capacity(map.len());
            for &v in map.values() {
                rt_retain(v);
                vals.push(v);
            }
            Rc::new(RefCell::new(GcData::List(vals)))
        } else {
            Rc::new(RefCell::new(GcData::List(vec![])))
        }
    };
    let _ = Rc::into_raw(rc);
    rc_to_ptr(list_rc)
}


#[no_mangle]
pub extern "C" fn rt_index_get(container: EncodedValue, key: EncodedValue) -> EncodedValue {
    if is_int(container) || container == 0 {
        return encode_int(0);
    }
    let rc = unsafe { Rc::from_raw(container as GcPtr) };
    let kind = {
        let borrow = rc.borrow();
        match &*borrow {
            GcData::List(_) => 0,
            GcData::Map(_) => 1,
            _ => 2,
        }
    };
    let _ = Rc::into_raw(rc);
    match kind {
        0 => rt_list_get(container, key),
        1 => rt_map_get(container, key),
        _ => encode_int(0),
    }
}

#[no_mangle]
pub extern "C" fn rt_index_set(container: EncodedValue, key: EncodedValue, value: EncodedValue) {
    if is_int(container) || container == 0 {
        return;
    }
    let rc = unsafe { Rc::from_raw(container as GcPtr) };
    let kind = {
        let borrow = rc.borrow();
        match &*borrow {
            GcData::List(_) => 0,
            GcData::Map(_) => 1,
            _ => 2,
        }
    };
    let _ = Rc::into_raw(rc);
    match kind {
        0 => rt_list_set(container, key, value),
        1 => rt_map_set(container, key, value),
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn rt_range(start: EncodedValue, end: EncodedValue) -> EncodedValue {
    let start_i = if is_int(start) { decode_int(start) } else { 0 };
    let end_i = if is_int(end) { decode_int(end) } else { 0 };
    let len = (end_i - start_i).max(0) as usize;
    let mut vals = Vec::with_capacity(len);
    let mut cur = start_i;
    while cur < end_i {
        vals.push(encode_int(cur));
        cur += 1;
    }
    rc_to_ptr(Rc::new(RefCell::new(GcData::List(vals))))
}


#[no_mangle]
pub extern "C" fn rt_file_open(path: EncodedValue) -> EncodedValue {
    let path_str = val_to_key(path);
    let file = File::open(path_str).ok();
    let rc = Rc::new(RefCell::new(GcData::File(file)));
    rc_to_ptr(rc)
}

#[no_mangle]
pub extern "C" fn rt_file_read(file_ptr: EncodedValue) -> EncodedValue {
    if is_int(file_ptr) || file_ptr == 0 {
        return rt_alloc_string("".as_ptr(), 0);
    }
    let rc = unsafe { Rc::from_raw(file_ptr as GcPtr) };
    let res = {
        let mut borrow = rc.borrow_mut();
        if let GcData::File(Some(f)) = &mut *borrow {
            let mut content = String::new();
            if f.read_to_string(&mut content).is_ok() {
                let s_rc = Rc::new(RefCell::new(GcData::String(content)));
                rc_to_ptr(s_rc)
            } else {
                rt_alloc_string("".as_ptr(), 0)
            }
        } else {
            rt_alloc_string("".as_ptr(), 0)
        }
    };
    let _ = Rc::into_raw(rc);
    res
}

#[no_mangle]
pub extern "C" fn rt_file_close(file_ptr: EncodedValue) {
    if is_int(file_ptr) || file_ptr == 0 {
        return;
    }
    let rc = unsafe { Rc::from_raw(file_ptr as GcPtr) };
    {
        let mut borrow = rc.borrow_mut();
        if let GcData::File(f) = &mut *borrow {
            *f = None; // Drop the file
        }
    }
    let _ = Rc::into_raw(rc);
}


#[no_mangle]
pub extern "C" fn rt_push_try(sp: u64, fp: u64, lr: u64) {
    EXCEPTION_STACK.with(|stack| {
        stack.borrow_mut().push(ExceptionHandler { sp, fp, lr });
    });
}

#[no_mangle]
pub extern "C" fn rt_pop_try() {
    EXCEPTION_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

#[no_mangle]
pub extern "C" fn rt_throw(val: EncodedValue) -> u64 {
    rt_retain(val);
    LAST_ERROR.with(|last| *last.borrow_mut() = val);

    let handler = EXCEPTION_STACK.with(|stack| stack.borrow().last().copied());

    if let Some(h) = handler {


        h.lr // Just return address for now, assuming SP restoration is hard.
    } else {
        println!("Uncaught exception: ");
        rt_print(val);
        std::process::exit(1);
    }
}

#[no_mangle]
pub extern "C" fn rt_get_last_error() -> EncodedValue {
    LAST_ERROR.with(|last| *last.borrow())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_abs_int() {
        let v = encode_int(-42);
        let out = rt_abs(v);
        assert!(is_int(out));
        assert_eq!(decode_int(out), 42);
    }

    #[test]
    fn test_rt_min_max_int() {
        let a = encode_int(5);
        let b = encode_int(9);
        assert_eq!(decode_int(rt_min(a, b)), 5);
        assert_eq!(decode_int(rt_max(a, b)), 9);
    }

    #[test]
    fn test_rt_contains_string() {
        let h = rt_alloc_string("hello world".as_ptr(), 11);
        let n = rt_alloc_string("world".as_ptr(), 5);
        let out = rt_contains(h, n);
        assert_eq!(decode_int(out), 1);
        rt_release(h);
        rt_release(n);
    }

    #[test]
    fn test_rt_range_and_pop() {
        let list = rt_range(encode_int(2), encode_int(5)); // [2,3,4]
        assert_eq!(decode_int(rt_list_len(list)), 3);
        let popped = rt_list_pop(list);
        assert_eq!(decode_int(popped), 4);
        assert_eq!(decode_int(rt_list_len(list)), 2);
        rt_release(list);
    }
}
