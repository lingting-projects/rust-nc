use log::LevelFilter;

pub fn is_enable_debug() -> bool {
    match log::max_level() {
        LevelFilter::Debug => true,
        LevelFilter::Trace => true,
        _ => false,
    }
}


pub fn is_enable_trace() -> bool {
    match log::max_level() {
        LevelFilter::Trace => true,
        _ => false,
    }
}