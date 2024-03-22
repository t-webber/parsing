#[macro_export]
macro_rules! define_enum_and_ref {
    ($name:ident, $nameref:ident, $namerefmut:ident, $($variant:ident($t:ty)),*) => {
        #[derive(Debug, Clone)]
        enum $name {
            $($variant($t),)*
        }

        #[derive(Debug)]
        enum $nameref<'to_ref> {
            $($variant(&'to_ref $t),)*
        }

        #[allow(dead_code)]
        #[derive(Debug)]
        enum $namerefmut<'to_ref> {
            $($variant(&'to_ref mut $t),)*
        }

        #[allow(clippy::pattern_type_mismatch)]
        const fn to_ref(elt: &$name) -> $nameref {
            match elt {
                $($name::$variant(content) => $nameref::$variant(content),)*
            }
        }

        #[allow(clippy::pattern_type_mismatch)]
        fn to_refmut(elt: &mut $name) -> $namerefmut {
            match elt {
                $($name::$variant(content) => $namerefmut::$variant(content),)*
            }
        }

    };
}
