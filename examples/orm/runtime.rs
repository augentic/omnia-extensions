//! ORM example runtime.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia_wasi_http::{WasiHttp, HttpDefault};
        use omnia_wasi_otel::{WasiOtel, OtelDefault};
        use omnia_wasi_sql::{WasiSql, SqlDefault};

        omnia::runtime!({
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiSql: SqlDefault,
            }
        });
    }
    _ => {
        fn main() {}
    }
}
