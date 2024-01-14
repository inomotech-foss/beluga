use super::{TargetFamily, TargetVendor};
use crate::Context;

#[derive(Debug)]
pub struct ThreadNameMethod {
    setter: Option<NameSetter>,
    getter: Option<NameGetter>,
}

impl ThreadNameMethod {
    pub fn detect(ctx: &Context) -> Self {
        eprintln!("detecting thread name methods");
        if matches!(ctx.target_family, TargetFamily::Windows) {
            // On Windows we do a runtime check for both getter and setter, instead of
            // compile-time check
            return Self {
                setter: None,
                getter: None,
            };
        }

        Self {
            setter: NameSetter::detect(ctx),
            getter: NameGetter::detect(ctx),
        }
    }

    pub fn apply(&self, build: &mut cc::Build) {
        if let Some(setter) = self.setter {
            build.define(setter.define_name(), None);
        }
        if let Some(getter) = self.getter {
            build.define(getter.define_name(), None);
        }
    }

    fn check_compiles(ctx: &Context, call: &str) -> bool {
        let code = format!(
            r#"
#define _GNU_SOURCE
#include <pthread.h>

#if defined(__FreeBSD__) || defined(__NetBSD__) || defined(__OpenBSD__)
#include <pthread_np.h>
#endif

int main() {{
    pthread_t thread_id;
    {call}
}}
"#
        );
        super::check_compiles(ctx, &code)
    }
}

#[derive(Clone, Copy, Debug)]
enum NameSetter {
    Setname2,
    Setname3,
    SetName2,
}

impl NameSetter {
    fn detect(ctx: &Context) -> Option<Self> {
        if matches!(ctx.target_vendor, TargetVendor::Apple) {
            // All Apple platforms we support have 1 arg version of the function.
            // So skip compile time check here and instead check if its apple in
            // the thread code.
            return None;
        }

        // pthread_setname_np() usually takes 2 args
        if ThreadNameMethod::check_compiles(ctx, r#"pthread_setname_np(thread_id, "asdf");"#) {
            return Some(Self::Setname2);
        }
        // OpenBSD's function takes 2 args, but has a different name.
        if ThreadNameMethod::check_compiles(ctx, r#"pthread_set_name_np(thread_id, "asdf");"#) {
            return Some(Self::SetName2);
        }
        // But on NetBSD it takes 3!
        if ThreadNameMethod::check_compiles(ctx, r#"pthread_setname_np(thread_id, "asdf", NULL);"#)
        {
            return Some(Self::Setname3);
        }

        // And on many older/weirder platforms it's just not supported
        // Consider using prctl if we really want to support those
        None
    }

    const fn define_name(self) -> &'static str {
        match self {
            Self::Setname2 => "AWS_PTHREAD_SETNAME_TAKES_2ARGS",
            Self::Setname3 => "AWS_PTHREAD_SETNAME_TAKES_3ARGS",
            Self::SetName2 => "AWS_PTHREAD_SET_NAME_TAKES_2ARGS",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum NameGetter {
    Getname3,
    Getname2,
    GetName2,
}

impl NameGetter {
    fn detect(ctx: &Context) -> Option<Self> {
        if matches!(ctx.target_vendor, TargetVendor::Apple) {
            // All Apple platforms we support have the same function, so no need for
            // compile-time check.
            return Some(Self::Getname3);
        }

        // Some platforms have 2 arg version
        if ThreadNameMethod::check_compiles(
            ctx,
            r#"char name[16] = {0}; pthread_getname_np(thread_id, name);"#,
        ) {
            return Some(Self::Getname2);
        }
        // Some platforms have 2 arg version but with a different name (eg, OpenBSD)
        if ThreadNameMethod::check_compiles(
            ctx,
            r#"char name[16] = {0}; pthread_get_name_np(thread_id, name);"#,
        ) {
            return Some(Self::GetName2);
        }
        // But majority have 3
        if ThreadNameMethod::check_compiles(
            ctx,
            r#"char name[16] = {0}; pthread_getname_np(thread_id, name, 16);"#,
        ) {
            return Some(Self::Getname3);
        }

        None
    }

    const fn define_name(self) -> &'static str {
        match self {
            Self::Getname3 => "AWS_PTHREAD_GETNAME_TAKES_3ARGS",
            Self::Getname2 => "AWS_PTHREAD_GETNAME_TAKES_2ARGS",
            Self::GetName2 => "AWS_PTHREAD_GET_NAME_TAKES_2ARGS",
        }
    }
}
