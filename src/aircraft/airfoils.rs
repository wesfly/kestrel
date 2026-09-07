use avian_fdm::{airfoil::foil_tools::parse_foil_tools_csv, prelude::AirfoilData};

/// The Breeze's main wing airfoil
pub fn ag47ct02r() -> AirfoilData {
    let csv: &str = include_str!("../../assets/airfoils/ag47ct02r_polars.csv");

    #[allow(clippy::expect_used)]
    parse_foil_tools_csv(csv)
        .expect("embedded ag47ct02r CSV to parse cleanly")
        .ncrit9
}

/// The Breeze's canard airfoil
pub fn naca0010() -> AirfoilData {
    let csv: &str = include_str!("../../assets/airfoils/naca0010_polars.csv");

    #[allow(clippy::expect_used)]
    parse_foil_tools_csv(csv)
        .expect("embedded NACA 0010 CSV to parse cleanly")
        .ncrit9
}

/// The J3Cub's airfoil
pub fn usa35b() -> AirfoilData {
    let csv: &str = include_str!("../../assets/airfoils/usa35b_polars.csv");

    #[allow(clippy::expect_used)]
    parse_foil_tools_csv(csv)
        .expect("embedded USA-35B CSV to parse cleanly")
        .ncrit9
}
