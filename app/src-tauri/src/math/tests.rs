use super::expr::*;
use super::*;

fn eval(s: &str) -> f64 {
    parser::parse(s).unwrap().eval0().unwrap()
}

#[test]
fn priorities_and_notation() {
    assert_eq!(eval("2 + 3 * 4"), 14.0);
    assert_eq!(eval("(2 + 3) * 4"), 20.0);
    assert_eq!(eval("2^3^2"), 512.0);
    assert_eq!(eval("-2^2"), -4.0);
    assert_eq!(eval("3,5 × 2"), 7.0);
    assert_eq!(eval("√16 + 2²"), 8.0);
    assert_eq!(eval("5!"), 120.0);
    assert_eq!(eval("|3 - 10|"), 7.0);
    assert_eq!(eval("log(8, 2)"), 3.0);
    assert_eq!(eval("(-8)^(1/3)"), -2.0);
    assert!((parser::parse("sin(30)").unwrap().eval(&Default::default(), AngleMode::Degrees).unwrap() - 0.5).abs() < 1e-12);
}

#[test]
fn errors() {
    assert!(parser::parse("1/0").unwrap().eval0().is_err());
    assert!(parser::parse("sqrt(-1)").unwrap().eval0().is_err());
    assert!(parser::parse("2 +").is_err());
    assert!(parser::parse("(2").is_err());
}

#[test]
fn formatting() {
    assert_eq!(fmt_num(0.1 + 0.2), "0.3");
    assert_eq!(fmt_num(2.0000000000001), "2");
    assert_eq!(fmt_num(-3.0), "-3");
    assert_eq!(fraction(0.75), Some((3, 4)));
    assert_eq!(fraction(std::f64::consts::PI), None);
}

#[test]
fn equations() {
    assert_eq!(solver::solve("2x + 3 = 7", AngleMode::Radians).unwrap().real_roots, vec![2.0]);
    assert_eq!(solver::solve("x^2 - 5x + 6 = 0", AngleMode::Radians).unwrap().real_roots, vec![2.0, 3.0]);
    let c = solver::solve("x^2 + 1 = 0", AngleMode::Radians).unwrap();
    assert_eq!(c.complex_roots, vec!["0 ± 1i".to_string()]);
    assert_eq!(solver::solve("x^3 - 6x^2 + 11x - 6 = 0", AngleMode::Radians).unwrap().real_roots, vec![1.0, 2.0, 3.0]);
    let t = solver::solve("cos(x) = x", AngleMode::Radians).unwrap();
    assert_eq!(t.real_roots.len(), 1);
    assert!((t.real_roots[0] - 0.739085133).abs() < 1e-6);
}

#[test]
fn systems() {
    let s = solver::solve_system(&["2x + y = 5".into(), "x - y = 1".into()]).unwrap();
    assert_eq!(s.values["x"], 2.0);
    assert_eq!(s.values["y"], 1.0);
    assert_eq!(solver::solve_system(&["x + y = 1".into(), "2x + 2y = 3".into()]).unwrap().status, solver::SystemStatus::None);
}

#[test]
fn derivatives() {
    let d = |s: &str| algebra::derivative(&parser::parse(s).unwrap(), "x").unwrap().to_string();
    assert_eq!(d("x^3"), "3x^2");
    assert_eq!(d("3x^2 + 2x + 1"), "6x + 2");
    assert_eq!(d("sin(x)"), "cos(x)");
}

#[test]
fn classification() {
    assert_eq!(classify("2+2"), Some(Request::Evaluate("2+2".into())));
    assert_eq!(classify("calcule 3 * (4 + 5)"), Some(Request::Evaluate("3 * (4 + 5)".into())));
    assert_eq!(classify("résous x^2 - 4 = 0"), Some(Request::Equation("x^2 - 4 = 0".into())));
    assert_eq!(classify("dérivée de x^2"), Some(Request::Derivative("x^2".into(), "x".into())));
    assert!(matches!(classify("2x + y = 5; x - y = 1"), Some(Request::System(_))));
    for prose in ["Bonjour, comment vas-tu ?", "Explique-moi la photosynthèse", "Qui est Cristiano Ronaldo", "Quel temps fait-il en 2024 ?"] {
        assert_eq!(classify(prose), None, "{prose}");
    }
}

#[test]
fn verified_output() {
    let md = execute(&Request::Equation("x^2 - 5x + 6 = 0".into()), AngleMode::Radians).unwrap();
    assert!(md.contains("x = 2") && md.contains('✓') && !md.contains('✗'));
}

#[test]
fn units() {
    let c = units::parse("10 km en miles").unwrap();
    assert!((c.result - 6.21371).abs() < 1e-4);
    assert!((units::parse("100 °F en °C").unwrap().result - 37.7778).abs() < 1e-3);
    assert!((units::parse("convertis 90 km/h en m/s").unwrap().result - 25.0).abs() < 1e-9);
    assert!(units::parse("10 km en kg").is_none());
    assert!(units::parse("bonjour en anglais").is_none());
}

#[test]
fn stats_and_matrices() {
    let s = stats::describe(&stats::parse_list("2, 4, 4, 4, 5, 5, 7, 9")).unwrap();
    assert_eq!(s.mean, 5.0);
    assert_eq!(s.std_dev, 2.0);
    let m = stats::parse_matrix("1 2; 3 4").unwrap();
    assert_eq!(stats::determinant(&m).unwrap(), -2.0);
    assert_eq!(stats::inverse(&m).unwrap(), vec![vec![-2.0, 1.0], vec![1.5, -0.5]]);
}

#[test]
fn spoken_operators() {
    assert_eq!(classify("Combien font 12 fois 7 ?"), Some(Request::Evaluate("12 * 7".into())));
    assert_eq!(classify("combien fait 10 divisé par 4"), Some(Request::Evaluate("10 / 4".into())));
    assert_eq!(classify("calcule 2 puissance 10 moins 24"), Some(Request::Evaluate("2 ^ 10 - 24".into())));
    assert_eq!(classify("Il y a 3 chats plus 2 chiens"), None);
}
