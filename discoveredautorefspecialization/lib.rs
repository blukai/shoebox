// NOTE: this is piece of an experiment that didn't survive, but i wanted to save an example of waht
// i discovered.

#[derive(Debug)]
struct OpaqueInfo {
    type_id: core::any::TypeId,
}

#[derive(Debug)]
enum TypeInfo {
    Opaque(OpaqueInfo),
}

trait Typed: 'static {
    const TYPE_INFO: &'static TypeInfo;
}

macro_rules! impl_opaque {
    ([$($generics:tt)*] $ty:ty) => {
        impl<$($generics)*> $crate::Typed for $ty {
            const TYPE_INFO: &'static $crate::TypeInfo = &$crate::TypeInfo::Opaque(
                $crate::OpaqueInfo {
                    type_id: core::any::TypeId::of::<Self>(),
                },
            );
        }
    };
    ($ty:ty) => {
        impl_opaque! { [] $ty }
    };
}

impl_opaque! { i8 }
impl_opaque! { u8 }
impl_opaque! { i16 }
impl_opaque! { u16 }
impl_opaque! { i32 }
impl_opaque! { u32 }
impl_opaque! { i64 }
impl_opaque! { u64 }
impl_opaque! { i128 }
impl_opaque! { u128 }

impl_opaque! { f32 }
impl_opaque! { f64 }

#[test]
fn test_typeinfo_specialization_demo() {
    // NOTE: the probe thing is a hack!
    //
    //   see
    //   https://github.com/dtolnay/case-studies/blob/f12517786bda4c131c6ea485d90a82276b39030c/autoref-specialization/README.md
    //
    //   route 1:
    //   ----
    //
    //   with
    //   impl<T: TypeInfo> ProbeTypeInfo for Probe<T> {}
    //   impl<T: 'static> ProbeOpaqueData for &Probe<T> {}
    //
    //   will resolve to specific type
    //   dbg!((&Probe::<u8>(core::marker::PhantomData)).ty());
    //   dbg!((Probe::<u8>(core::marker::PhantomData)).ty());
    //
    //   but this will resolve to opaque
    //   dbg!((&&Probe::<u8>(core::marker::PhantomData)).ty());
    //
    //   route 2:
    //   ----
    //
    //   if you swap the &
    //   impl<T: TypeInfo> ProbeTypeInfo for &Probe<T> {}
    //   impl<T: 'static> ProbeOpaqueData for Probe<T> {}
    //
    //   slaping more references &&&&&&&&& (need at least 2) would guarantee that the prob would resolve
    //   the specific ty first.

    struct OpaqueData<T>(core::marker::PhantomData<T>);

    impl<T: 'static> OpaqueData<T> {
        const TYPE_INFO: &'static TypeInfo = &TypeInfo::Opaque(OpaqueInfo {
            type_id: core::any::TypeId::of::<T>(),
        });
    }

    struct Probe<T>(core::marker::PhantomData<T>);

    trait ProbeTypeInfo {
        fn type_info(&self) -> &'static TypeInfo;
    }

    impl<T: Typed> ProbeTypeInfo for Probe<T> {
        fn type_info(&self) -> &'static TypeInfo {
            T::TYPE_INFO
        }
    }

    trait ProbeOpaqueData {
        fn type_info(&self) -> &'static TypeInfo;
    }

    impl<T: 'static> ProbeOpaqueData for &Probe<T> {
        fn type_info(&self) -> &'static TypeInfo {
            OpaqueData::<T>::TYPE_INFO
        }
    }

    dbg!((&&&Probe::<u8>(core::marker::PhantomData)).type_info());
    dbg!((&&Probe::<u8>(core::marker::PhantomData)).type_info());
    dbg!((&Probe::<u8>(core::marker::PhantomData)).type_info());
    dbg!((Probe::<u8>(core::marker::PhantomData)).type_info());

    // NOTE: i couldn't use this the way i was imagining, stupido.
    //
    //   {
    //       use $crate::{ProbeTypeInfo, ProbeOpaqueData};
    //       (&$crate::Probe::<$field_ty>(core::marker::PhantomData)).ty()
    //   }
    //
    //   QUOTE:
    //     > cannot call non-const method <Probe<u8> as ProbeTypeInfo>::ty in constants
    //     - compiler
}
