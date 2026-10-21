//! HWP 파일 헤더 파싱
//!
//! FileHeader 스트림 구조 (256바이트, 비압축):
//! - 0~31:  시그니처 ("HWP Document File" + NULL 패딩)
//! - 32~35: 버전 (revision, build, minor, major) LE
//! - 36~39: 속성 플래그 (u32 LE)
//! - 40~43: 라이선스 속성
//! - 44~47: EncryptVersion (u32 LE)
//! - 48~255: 예약

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// HWP 파일 시그니처
pub const HWP_SIGNATURE: &[u8] = b"HWP Document File";

/// FileHeader 크기 (바이트)
pub const FILE_HEADER_SIZE: usize = 256;

/// HWP 버전 정보
#[derive(Debug, Clone, PartialEq)]
pub struct HwpVersion {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub revision: u8,
}

impl HwpVersion {
    /// 지원되는 버전인지 확인 (5.0, 5.1)
    pub fn is_supported(&self) -> bool {
        self.major == 5 && (self.minor == 0 || self.minor == 1)
    }
}

impl std::fmt::Display for HwpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.build, self.revision
        )
    }
}

/// FileHeader 속성 플래그
#[derive(Debug, Clone)]
pub struct FileHeaderFlags {
    /// 원본 플래그 값
    pub raw: u32,
    /// 압축 여부
    pub compressed: bool,
    /// 암호화 여부
    pub encrypted: bool,
    /// 배포용 문서
    pub distribution: bool,
    /// 스크립트 저장
    pub script: bool,
    /// DRM 보안
    pub drm: bool,
    /// XML 템플릿 저장
    pub xml_template: bool,
    /// 문서 이력 관리
    pub document_history: bool,
    /// 전자 서명
    pub digital_signature: bool,
    /// 공개키 암호화
    pub public_key_encrypted: bool,
    /// 수정 인증서
    pub modified_certificate: bool,
    /// 배포 준비
    pub prepare_distribution: bool,
}

/// 한글이 본문 스토리지를 `ViewText` 로 고르는 FileHeader 속성 비트의 합 —
/// 배포용 `0x4` · DRM `0x10` · 공인 인증서 DRM `0x400` · 변경 추적 `0x4000`.
///
/// 맥 한글 12.30(`HwpApp.dylib` arm64) 디컴파일: 열기 검증 `FUN_004f67d8` 이 `flags & 0x4414` 일 때만
/// `ViewText` 를, 아니면 `BodyText` 를 연다. 실제 로더 `FUN_00503c84` 도 같은 범주로 ViewText 로더
/// (`FUN_00504730`)와 일반 로더를 가르고, 저장 `FUN_004fd1a0` 은 그런 문서의 `BodyText` 에 안내 문구
/// 스텁만 쓰고 진짜 본문을 `ViewText` 에 쓴다. 이 비트가 하나도 없으면 `ViewText` 는 무시된다 —
/// 변경 추적 문서를 일반 문서로 바꿔 저장한 뒤 남은 잔재가 그런 경우다.
pub const VIEWTEXT_BODY_FLAGS: u32 = 0x4414;

impl FileHeaderFlags {
    /// 한글이 이 문서의 본문을 `ViewText` 에서 여는가(아니면 `BodyText`).
    pub fn viewtext_is_body(&self) -> bool {
        self.raw & VIEWTEXT_BODY_FLAGS != 0
    }

    /// u32 플래그 값에서 파싱
    pub fn from_u32(flags: u32) -> Self {
        FileHeaderFlags {
            raw: flags,
            compressed: (flags & 0x01) != 0,
            encrypted: (flags & 0x02) != 0,
            distribution: (flags & 0x04) != 0,
            script: (flags & 0x08) != 0,
            drm: (flags & 0x10) != 0,
            xml_template: (flags & 0x20) != 0,
            document_history: (flags & 0x40) != 0,
            digital_signature: (flags & 0x80) != 0,
            public_key_encrypted: (flags & 0x100) != 0,
            modified_certificate: (flags & 0x200) != 0,
            prepare_distribution: (flags & 0x400) != 0,
        }
    }
}

/// 파싱된 FileHeader
#[derive(Debug, Clone)]
pub struct FileHeader {
    pub version: HwpVersion,
    pub flags: FileHeaderFlags,
    /// 비밀번호 암호화 방식 버전. 4는 한글 7.0 이후 방식이다.
    pub encrypt_version: u32,
}

/// FileHeader 파싱 에러
#[derive(Debug)]
pub enum HeaderError {
    TooShort(usize),
    InvalidSignature,
    UnsupportedVersion(HwpVersion),
    IoError(String),
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::TooShort(size) => {
                write!(
                    f,
                    "FileHeader 크기 부족: {} (최소 {})",
                    size, FILE_HEADER_SIZE
                )
            }
            HeaderError::InvalidSignature => write!(f, "HWP 시그니처가 일치하지 않습니다"),
            HeaderError::UnsupportedVersion(v) => write!(f, "지원하지 않는 HWP 버전: {}", v),
            HeaderError::IoError(e) => write!(f, "FileHeader 읽기 오류: {}", e),
        }
    }
}

impl std::error::Error for HeaderError {}

/// FileHeader 바이너리 데이터 파싱
pub fn parse_file_header(data: &[u8]) -> Result<FileHeader, HeaderError> {
    if data.len() < FILE_HEADER_SIZE {
        return Err(HeaderError::TooShort(data.len()));
    }

    // 시그니처 검증 (0~31, NULL 패딩 제거 후 비교)
    let sig_area = &data[0..32];
    let sig_end = sig_area.iter().position(|&b| b == 0).unwrap_or(32);
    let signature = &sig_area[..sig_end];

    if !signature.starts_with(HWP_SIGNATURE) {
        return Err(HeaderError::InvalidSignature);
    }

    // 버전 (32~35): revision, build, minor, major (LE)
    let version = HwpVersion {
        revision: data[32],
        build: data[33],
        minor: data[34],
        major: data[35],
    };

    // 속성 플래그 (36~39)
    let mut cursor = Cursor::new(&data[36..40]);
    let flags_raw = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| HeaderError::IoError(e.to_string()))?;
    let flags = FileHeaderFlags::from_u32(flags_raw);

    // EncryptVersion (44~47)
    let mut cursor = Cursor::new(&data[44..48]);
    let encrypt_version = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| HeaderError::IoError(e.to_string()))?;

    Ok(FileHeader {
        version,
        flags,
        encrypt_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트용 FileHeader 바이트 생성
    fn make_file_header(major: u8, minor: u8, flags: u32) -> Vec<u8> {
        let mut data = vec![0u8; FILE_HEADER_SIZE];

        // 시그니처
        data[..HWP_SIGNATURE.len()].copy_from_slice(HWP_SIGNATURE);

        // 버전 (revision, build, minor, major)
        data[32] = 0; // revision
        data[33] = 0; // build
        data[34] = minor;
        data[35] = major;

        // 플래그
        data[36..40].copy_from_slice(&flags.to_le_bytes());

        data
    }

    #[test]
    fn test_hwp_signature() {
        assert_eq!(HWP_SIGNATURE, b"HWP Document File");
    }

    #[test]
    fn test_parse_valid_header() {
        let data = make_file_header(5, 0, 0x01); // v5.0, compressed

        let header = parse_file_header(&data).unwrap();
        assert_eq!(header.version.major, 5);
        assert_eq!(header.version.minor, 0);
        assert!(header.flags.compressed);
        assert!(!header.flags.encrypted);
        assert!(!header.flags.distribution);
    }

    #[test]
    fn test_parse_distribution_document() {
        let data = make_file_header(5, 0, 0x05); // compressed + distribution

        let header = parse_file_header(&data).unwrap();
        assert!(header.flags.compressed);
        assert!(header.flags.distribution);
        assert!(!header.flags.encrypted);
    }

    /// 맥 한글 12.30(REA `HwpApp.dylib` arm64): 열기 검증 `FUN_004f67d8` 은 속성 `flags & 0x4414` 일 때만
    /// 본문 스토리지를 `ViewText` 로 고르고 아니면 `BodyText` 로 고른다. 실제 로더 `FUN_00503c84` 도 같은
    /// 범주(배포용 · DRM · 공인DRM · 변경추적)로 ViewText 로더와 일반 로더를 가른다.
    #[test]
    fn viewtext_is_body_follows_hancom_mask() {
        assert_eq!(VIEWTEXT_BODY_FLAGS, 0x4414);
        // 켜지는 비트: 배포용 0x4 · DRM 0x10 · 공인DRM 0x400 · 변경추적 0x4000
        for bit in [0x4u32, 0x10, 0x400, 0x4000] {
            assert!(
                FileHeaderFlags::from_u32(0x01 | bit).viewtext_is_body(),
                "속성 +0x{bit:x} 는 ViewText 를 본문으로 연다"
            );
        }
        // 그 밖의 비트는 하나씩 켜도 BodyText 가 본문이다(ViewText 는 잔재).
        for bit in (0..32)
            .map(|i| 1u32 << i)
            .filter(|bit| bit & VIEWTEXT_BODY_FLAGS == 0)
        {
            assert!(
                !FileHeaderFlags::from_u32(0x01 | bit).viewtext_is_body(),
                "속성 +0x{bit:x} 는 ViewText 를 본문으로 열지 않는다"
            );
        }
        assert!(!FileHeaderFlags::from_u32(0x01).viewtext_is_body());
        // 상류 #5169 표본(외교부 연구계획서)은 변경추적 0x4001 이다.
        assert!(FileHeaderFlags::from_u32(0x4001).viewtext_is_body());
    }

    #[test]
    fn test_parse_encrypted_document() {
        let mut data = make_file_header(5, 0, 0x03); // compressed + encrypted
        data[44..48].copy_from_slice(&4u32.to_le_bytes());

        let header = parse_file_header(&data).unwrap();
        assert!(header.flags.compressed);
        assert!(header.flags.encrypted);
        assert_eq!(header.encrypt_version, 4);
    }

    #[test]
    fn test_parse_encrypt_version_is_little_endian() {
        let mut data = make_file_header(5, 0, 0);
        data[44..48].copy_from_slice(&0x0102_0304u32.to_le_bytes());

        let header = parse_file_header(&data).unwrap();
        assert_eq!(header.encrypt_version, 0x0102_0304);
    }

    #[test]
    fn test_parse_all_flags() {
        let data = make_file_header(5, 1, 0x7FF); // 모든 플래그 ON

        let header = parse_file_header(&data).unwrap();
        assert!(header.flags.compressed);
        assert!(header.flags.encrypted);
        assert!(header.flags.distribution);
        assert!(header.flags.script);
        assert!(header.flags.drm);
        assert!(header.flags.xml_template);
        assert!(header.flags.document_history);
        assert!(header.flags.digital_signature);
        assert!(header.flags.public_key_encrypted);
        assert!(header.flags.modified_certificate);
        assert!(header.flags.prepare_distribution);
    }

    #[test]
    fn test_too_short_data() {
        let data = vec![0u8; 100];
        let result = parse_file_header(&data);
        assert!(matches!(result, Err(HeaderError::TooShort(100))));
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = vec![0u8; FILE_HEADER_SIZE];
        data[..10].copy_from_slice(b"NOT A HWP!");
        let result = parse_file_header(&data);
        assert!(matches!(result, Err(HeaderError::InvalidSignature)));
    }

    #[test]
    fn test_version_display() {
        let v = HwpVersion {
            major: 5,
            minor: 0,
            build: 6,
            revision: 1,
        };
        assert_eq!(format!("{}", v), "5.0.6.1");
    }

    #[test]
    fn test_version_supported() {
        assert!(HwpVersion {
            major: 5,
            minor: 0,
            build: 0,
            revision: 0
        }
        .is_supported());
        assert!(HwpVersion {
            major: 5,
            minor: 1,
            build: 0,
            revision: 0
        }
        .is_supported());
        assert!(!HwpVersion {
            major: 3,
            minor: 0,
            build: 0,
            revision: 0
        }
        .is_supported());
    }
}
