//! 글자 장식(밑줄) 기하 — 한글 2022 실측 계약. (#5730)
//!
//! 종전에는 밑줄을 기준선 아래 **고정 2.0px** 에 그렸다. 한글 2022 COM 프로브
//! (9/12/15/18/24/36pt, 실선)로 실측하면 밑줄은 글꼴 크기에 비례해
//! **기준선 + 0.17em** 에 온다 (1.44/9=0.160, 2.04/12=0.170, 2.52/15=0.168,
//! 3.00/18=0.167, 4.07/24=0.170, 6.11/36=0.170). 고정 2.0px 은 11.8px 글꼴에서만
//! 우연히 맞고, 제목처럼 큰 글꼴에서는 밑줄이 디센더를 가로지른다
//! (156467175 실측: 24px 제목에서 한글 4.2px vs rhwp 0.8px).
//!
//! 이중/삼중선(shape 7~10)도 같은 프로브(15pt·24pt)로 선별 위치·굵기가 전부
//! em 비례임을 실측했다. 이 모듈이 그 계약의 단일 출처다 — SVG/Skia 백엔드가
//! 같은 표를 소비한다.

/// 단일선(및 파선 등 shape 0~6, 11)의 기준선 아래 오프셋 비율 (em).
pub(crate) const UNDERLINE_BASELINE_RATIO: f64 = 0.17;

/// 밑줄 다중선(shape 7~10)의 (기준선 아래 오프셋 em, 선 굵기 em) 목록.
///
/// 한글 2022 실측 (15pt/24pt 프로브, PDF 드로잉 좌표):
/// - 7 이중선: 0.160(0.032) + 0.250(0.032)
/// - 8 가는+굵은: 0.175(0.048) + 0.300(0.100)
/// - 9 굵은+가는: 0.200(0.100) + 0.325(0.048)
/// - 10 삼중선: 0.165(0.032) + 0.250(0.080) + 0.335(0.032)
///
/// 그 외 shape 는 `None` — 호출부가 단일선 경로(0.17em, 기존 선 모양 유지)로 그린다.
pub(crate) fn underline_multi_lines(shape: u8) -> Option<&'static [(f64, f64)]> {
    match shape {
        7 => Some(&[(0.160, 0.032), (0.250, 0.032)]),
        8 => Some(&[(0.175, 0.048), (0.300, 0.100)]),
        9 => Some(&[(0.200, 0.100), (0.325, 0.048)]),
        10 => Some(&[(0.165, 0.032), (0.250, 0.080), (0.335, 0.032)]),
        _ => None,
    }
}

/// 밑줄 굵기(px) — 맥 한글 12.30 의 비례 방식.
///
/// 굵기는 글자 크기에 비례하는 **정수 HU** 다: `T = round(m × 크기HU ÷ 1000)`.
/// m 은 모양별 정수 — 단선·파선(0~6) 39(≈0.039em), 이중선(7)·물결(11·12) 113(≈0.113em),
/// 가는-굵은(8)·굵은-가는(9)·삼중선(10) 198(≈0.198em). 다중선·물결의 값은 «띠 전체»
/// 두께이고 그 안을 나누는 법은 호출부 몫이다. 우리는 종전 1px(0.75pt) 고정이라 10pt
/// 본문 밑줄이 맥(0.36pt)의 2.1배였고 24pt 에서는 0.78배였다(맥 PDF 61 고유 스팬,
/// ±0.12pt 일치 9/61 → 61/61).
///
/// 맥은 «크기»에 줄의 글자 크기(줄 안 최대)를 쓰는데 호출부는 run 의 기본 글자 크기
/// (첨자·장평 보정 전)를 넘긴다 — 한 줄이 한 크기이면 같고(59/61), 큰 글자와 섞인 줄의
/// 작은 글자 밑줄만 얇게 나온다. `size_px` 가 0 이하거나 유한하지 않으면 0.
pub(crate) fn underline_thickness_px(size_px: f64, shape: u8) -> f64 {
    if !size_px.is_finite() || size_px <= 0.0 {
        return 0.0;
    }
    let per_mille_hu = match shape {
        7 | 11 | 12 => 113.0,
        8..=10 => 198.0,
        _ => 39.0,
    };
    // 96dpi px → HU (1px = 75HU)
    (per_mille_hu * size_px * 75.0 / 1000.0).round() / 75.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 맥 한글 PDF 단일 밑줄의 굵기(pt) — 장치 눈금 0.12pt 로 양자화된 값이다.
    /// (글자 크기 pt, 맥 굵기 pt). 61 고유 스팬의 크기별 평균.
    const MAC_SINGLE_UNDERLINE_PT: &[(f64, f64)] = &[
        (10.0, 0.36),
        (11.0, 0.48),
        (13.0, 0.48),
        (15.0, 0.60),
        (16.0, 0.60),
        (18.0, 0.72),
        (20.0, 0.84),
        (24.0, 0.96),
    ];

    fn pt_to_px(pt: f64) -> f64 {
        pt * 96.0 / 72.0
    }

    fn px_to_pt(px: f64) -> f64 {
        px * 72.0 / 96.0
    }

    #[test]
    fn td01_single_underline_thickness_matches_mac_pdf_within_one_device_tick() {
        for &(size_pt, mac_pt) in MAC_SINGLE_UNDERLINE_PT {
            for shape in 0..=6u8 {
                let got = px_to_pt(underline_thickness_px(pt_to_px(size_pt), shape));
                assert!(
                    (got - mac_pt).abs() <= 0.12 + 1e-9,
                    "{size_pt}pt 모양 {shape}: 굵기 {got:.3}pt, 맥 {mac_pt:.2}pt"
                );
            }
        }
    }

    /// 굵기는 정수 HU 로 반올림한다 — T = round(39 × 크기HU ÷ 1000) (모양 0~6).
    #[test]
    fn td01_single_underline_thickness_is_integer_hwpunit() {
        // 10pt = 1000HU → 39HU, 12pt = 1200HU → 46.8 → 47HU, 16pt = 1600HU → 62.4 → 62HU
        for (size_hu, want_hu) in [(1000.0, 39.0), (1200.0, 47.0), (1600.0, 62.0)] {
            let size_px = size_hu / 75.0;
            let got_hu = underline_thickness_px(size_px, 0) * 75.0;
            assert!(
                (got_hu - want_hu).abs() < 1e-6,
                "{size_hu}HU: {got_hu} != {want_hu}"
            );
        }
    }

    /// 이중선(7)·가는-굵은(8)은 맥 PDF 띠 총 두께와 같다 — 11pt 이중선 1.24pt, 20pt 가는-굵은 3.96pt.
    #[test]
    fn td01_multi_line_total_band_matches_mac_pdf() {
        let double_11 = px_to_pt(underline_thickness_px(pt_to_px(11.0), 7));
        assert!((double_11 - 1.24).abs() < 0.005, "{double_11}");
        let thin_thick_20 = px_to_pt(underline_thickness_px(pt_to_px(20.0), 8));
        assert!((thin_thick_20 - 3.96).abs() < 0.005, "{thin_thick_20}");
    }

    #[test]
    fn td01_thickness_is_zero_for_nonpositive_size() {
        assert_eq!(underline_thickness_px(0.0, 0), 0.0);
        assert_eq!(underline_thickness_px(-3.0, 0), 0.0);
        assert_eq!(underline_thickness_px(f64::NAN, 0), 0.0);
    }
}
