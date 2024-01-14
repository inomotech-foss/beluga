use crate::Context;

#[derive(Debug)]
pub struct Simd {
    flags: Avx2Flags,
    pub have_avx2_intrinsics: bool,
    pub have_mm256_extract_epi64: bool,
}

impl Simd {
    pub fn detect(ctx: &Context) -> Self {
        eprintln!("detecting SIMD support");
        let flags = Avx2Flags::detect(ctx);

        let mut tmp_build = ctx.cc_build.clone();
        flags.apply(&mut tmp_build);
        let have_avx2_intrinsics = super::check_compiles_with_cc(
            ctx,
            &mut tmp_build,
            r#"
#include <immintrin.h>
#include <emmintrin.h>
#include <string.h>

int main() {
    __m256i vec;
    memset(&vec, 0, sizeof(vec));

    _mm256_shuffle_epi8(vec, vec);
    _mm256_set_epi32(1,2,3,4,5,6,7,8);
    _mm256_permutevar8x32_epi32(vec, vec);

    return 0;
}
"#,
        );
        let have_mm256_extract_epi64 = super::check_compiles_with_cc(
            ctx,
            &mut tmp_build,
            r#"
#include <immintrin.h>
#include <string.h>

int main() {
    __m256i vec;
    memset(&vec, 0, sizeof(vec));
    return (int)_mm256_extract_epi64(vec, 2);
}
"#,
        );

        Self {
            flags,
            have_avx2_intrinsics,
            have_mm256_extract_epi64,
        }
    }

    pub fn apply_defines(&self, build: &mut cc::Build) {
        if self.have_avx2_intrinsics {
            build.define("HAVE_AVX2_INTRINSICS", None);
        }
        if self.have_mm256_extract_epi64 {
            build.define("HAVE_MM256_EXTRACT_EPI64", None);
        }
    }

    pub fn apply_flags(&self, build: &mut cc::Build) {
        self.flags.apply(build);
    }
}

#[derive(Clone, Copy, Debug)]
enum Avx2Flags {
    None,
    Msvc,
    Gnu,
}

impl Avx2Flags {
    fn detect(ctx: &Context) -> Self {
        if ctx.compiler.is_like_msvc() {
            if ctx
                .cc_build
                .is_flag_supported("/arch:AVX2")
                .expect("check avx2 flag support")
            {
                return Self::Msvc;
            }
        } else if ctx
            .cc_build
            .is_flag_supported("-mavx2")
            .expect("check avx2 flag support")
        {
            return Self::Gnu;
        }

        Self::None
    }

    fn apply(self, build: &mut cc::Build) {
        match self {
            Self::None => {}
            Self::Msvc => {
                build.flag("/arch:AVX2");
            }
            Self::Gnu => {
                build.flag("-mavx").flag("-mavx2");
            }
        }
    }
}
