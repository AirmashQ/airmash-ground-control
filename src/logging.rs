/// Helper macro to simply warn about an error from
/// a result. Otherwise, do nothing.
#[macro_export]
macro_rules! warn_on_err {
    ($res:expr) => {
        if let Err(err) = $res {
            log::warn!("{}", err);
        }
    };
}
