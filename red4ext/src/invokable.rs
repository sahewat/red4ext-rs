use std::mem;

use red4ext_sys::ffi;
use red4ext_sys::interop::{Mem, StackArg};

use crate::conv::{FromRED, IntoRED};
use crate::rtti;
use crate::types::{CName, Ref, VoidPtr};

pub type REDFunction = unsafe extern "C" fn(*mut ffi::IScriptable, *mut ffi::CStackFrame, Mem, i64);
type REDType = *const ffi::CBaseRTTIType;

pub trait REDInvokable<A, R> {
    const ARG_TYPES: &'static [CName];
    const RETURN_TYPE: CName;

    fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem);
}

macro_rules! impl_invokable {
    ($( $types:ident ),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<$($types,)* R, FN> REDInvokable<($($types,)*), R> for FN
        where
            FN: Fn($($types,)*) -> R,
            $($types: FromRED,)*
            R: IntoRED
        {
            const ARG_TYPES: &'static [CName] = &[$(CName::new($types::NAME),)*];
            const RETURN_TYPE: CName = CName::new(R::NAME);

            #[inline]
            fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem) {
                $(let $types = FromRED::from_red(frame);)*
                let res = self($($types,)*);
                IntoRED::into_red(res, mem);
            }
        }
    };
}

impl_invokable!();
impl_invokable!(A);
impl_invokable!(A, B);
impl_invokable!(A, B, C);
impl_invokable!(A, B, C, D);
impl_invokable!(A, B, C, D, E);
impl_invokable!(A, B, C, D, E, F);

#[inline]
pub fn invoke<R: FromRED, const N: usize>(
    this: Ref<ffi::IScriptable>,
    fun: *mut ffi::CBaseFunction,
    args: [StackArg; N],
) -> R {
    let mut ret = R::Repr::default();
    unsafe {
        ffi::execute_function(
            VoidPtr(this.instance as _),
            fun,
            mem::transmute(&mut ret),
            &args,
        )
    };
    R::from_repr(&ret)
}

#[inline]
pub fn into_repr_and_type<A: IntoRED>(val: A) -> (REDType, A::Repr) {
    (rtti::get_type(CName::new(A::NAME)), val.into_repr())
}

#[macro_export]
macro_rules! invoke {
    ($this:expr, $func:expr, ($( $args:expr ),*) -> $rett:ty) => {
        {
            let args = ($($crate::invokable::into_repr_and_type($args)),*);
            let res: $rett = $crate::invokable::invoke($this, $func, $crate::invokable::Args::to_stack_args(&args));
            res
        }
    };
}

#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $crate::types::Ref::null(),
            $crate::rtti::get_function($crate::types::CName::new($fn_name)),
            ($($args),*) -> $rett
        )
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $this,
            $crate::rtti::get_method($this, $crate::types::CName::new($fn_name)),
            ($($args),*) -> $rett
        )
    };
}

pub trait Args {
    type StackArgs;

    fn to_stack_args(&self) -> Self::StackArgs;
}

macro_rules! impl_args {
    ($($ids:ident),*) => {
        #[allow(unused_parens, non_snake_case)]
        impl <$($ids),*> Args for ($((REDType, $ids)),*) {
            type StackArgs = [StackArg; count_args!($($ids)*)];

            #[inline]
            fn to_stack_args(&self) -> Self::StackArgs {
                let ($($ids),*) = self;
                [$(StackArg::new($ids.0, &$ids.1 as *const _ as _)),*]
            }
        }
    };
}

macro_rules! count_args {
    ($id:ident $($t:tt)*) => {
        1 + count_args!($($t)*)
    };
    () => { 0 }
}

impl_args!();
impl_args!(A);
impl_args!(A, B);
impl_args!(A, B, C);
impl_args!(A, B, C, D);
impl_args!(A, B, C, D, E);
impl_args!(A, B, C, D, E, F);
