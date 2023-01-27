use std::rc::Rc;

// Error

#[derive(Debug, Clone)]
pub enum EvalError {
    TypeMismatch(&'static str),
    UndefinedName(String),
    Misc(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for EvalError {}

pub type EvalResult<B> = Result<Rc<Val<B>>, EvalError>;

// Namespace

/// `Ns` stands for 'Namespace'.
pub struct Ns<B> {
    current: im::HashMap<Rc<String>, Rc<Val<B>>>,
    parent: Option<Rc<Ns<B>>>,
}

impl<B> Ns<B> {
    pub fn new() -> Self {
        Ns {
            current: im::HashMap::new(),
            parent: None,
        }
    }

    pub fn child(self: Rc<Self>) -> Self {
        Ns {
            current: im::HashMap::new(),
            parent: Some(self),
        }
    }

    pub fn lookup(&self, key: Rc<String>) -> Option<Rc<Val<B>>> {
        let pc = self.parent.clone();
        self.current
            .get(&key)
            .cloned()
            .or_else(|| pc.and_then(|p| p.lookup(key)))
    }

    pub fn update(&self, key: Rc<String>, value: Rc<Val<B>>) -> Self {
        Ns {
            current: self.current.update(key, value),
            parent: self.parent.clone(),
        }
    }

    /// Convenience method for populating a "Library scope" or a "Built-in scope"
    pub fn entry(&self, name: &'static str, value: Val<B>) -> Self {
        self.update(Rc::new(name.to_string()), Rc::new(value))
    }
}

impl<B> Default for Ns<B> {
    fn default() -> Self {
        Self::new()
    }
}

// Values and expressions

/// Value, a result of an expression; a unit of data in a program.
///
/// The `B` parameter stands for "built-in object". Different execution
/// strategies will require different type guarantees for their built-in functions.
///
/// For example, a `B`uiltin for a simple tree-walking interpreter might look like this:
///
/// ```rust
/// pub type MyVal = Val<MyBuiltin>;
/// pub type MyEvalResult = EvalResult<Val>;
/// pub struct MyBuiltin(Rc<Box<dyn Fn(Rc<MyVal>) -> MyEvalResult>>);
/// ```
///
/// If you're going for some kind of blazing fast™ but limited interpreter, you might
/// opt in for simple integers for which you'll look up operations in a table.
///
/// ```rust
/// pub struct MyBuiltin(u16);
/// ```
///
/// If you're doing something more complicated (see the [Hmm Evaluator][`crate::hmm_eval`] strategy):
/// ```rust
/// pub struct Builtin(Callback);
///
/// type Callback = Box<dyn FnOnce(Rc<Val>) -> Thunk>;
///
/// enum Thunk {
///     Done(EvalResult),
///     NeedElaboration(Rc<Expr>, Rc<Ns>, Callback),
/// }
/// ```
///
pub enum Val<B> {
    Int(i64),
    Str(Rc<String>),
    Vec(im::Vector<Rc<Val<B>>>),
    Builtin(&'static str, B),
    UserFn {
        argname: Rc<String>,
        body: Rc<Expr<B>>,
        ns: Rc<Ns<B>>,
    },
}

impl<B> std::fmt::Debug for Val<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Int(i) => f.debug_tuple("Int").field(i).finish(),
            Val::Str(s) => f.debug_tuple("Str").field(s).finish(),
            Val::Vec(v) => f.debug_tuple("Vec").field(v).finish(),
            Val::Builtin(name, _) => f
                .debug_tuple("BuiltinFn")
                .field(name)
                .field(&"...")
                .finish(),
            Val::UserFn {
                argname,
                body: _,
                ns: _,
            } => f
                .debug_tuple("UserFn")
                .field(argname)
                .field(&"...")
                .finish(),
        }
    }
}

pub enum Expr<B> {
    Put(Rc<Val<B>>),
    Name(Rc<String>),
    Call {
        func: Rc<Expr<B>>,
        arg: Rc<Expr<B>>,
    },
    Lam {
        argname: Rc<String>,
        body: Rc<Expr<B>>,
    },
}

// XXX: Manually implement `Debug` because `#[derive(Debug)]` puts a `B: Debug` requirement
//      See https://github.com/rust-lang/rust/issues/26925
impl<B> std::fmt::Debug for Expr<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Put(arg0) => f.debug_tuple("Put").field(arg0).finish(),
            Self::Name(arg0) => f.debug_tuple("Name").field(arg0).finish(),
            Self::Call { func, arg } => f
                .debug_struct("Call")
                .field("func", func)
                .field("arg", arg)
                .finish(),
            Self::Lam { argname, body } => f
                .debug_struct("Lam")
                .field("argname", argname)
                .field("body", body)
                .finish(),
        }
    }
}

// Conversions for the `F!` macro

impl<B> From<i64> for Val<B> {
    fn from(i: i64) -> Self {
        Val::Int(i)
    }
}

impl<B> From<String> for Val<B> {
    fn from(s: String) -> Self {
        Val::Str(Rc::new(s))
    }
}

impl<B> From<&'static str> for Val<B> {
    fn from(s: &'static str) -> Self {
        s.to_string().into()
    }
}

impl<T: Into<Val<B>>, B> From<Vec<T>> for Val<B> {
    fn from(vec: Vec<T>) -> Self {
        let vec: Vec<Rc<Val<B>>> = vec.into_iter().map(|t| Rc::new(t.into())).collect();
        Val::Vec(im::Vector::from(vec))
    }
}

/// Handy macro for writing expressions.
///
/// Input:
/// ```rust
/// F!(
///     'app F!('app F![lie], 'to F![420]),
///     'to F!(
///         'app F!('app F![tru], 'to F![55]),
///         'to F!('lam x . F!('app F![x], 'to F![lie]))));
/// ```
/// Is the same as:
/// ```rust
/// Rc::new(Expr::Call {
///     func: Rc::new(Expr::Call {
///         func: Rc::new(
///             Expr::Name(Rc::new(("lie").into())),
///         ),
///         arg: Rc::new(Expr::Put(Rc::new((420).into()))),
///     }),
///     arg: Rc::new(Expr::Call {
///         func: Rc::new(Expr::Call {
///             func: Rc::new(
///                 Expr::Name(Rc::new(("tru").into())),
///             ),
///             arg: Rc::new(Expr::Put(Rc::new((55).into()))),
///         }),
///         arg: Rc::new(Expr::Lam {
///             argname: Rc::new(("x").into()),
///             body: Rc::new(Expr::Call {
///                 func: Rc::new(
///                     Expr::Name(Rc::new(("x").into())),
///                 ),
///                 arg: Rc::new(
///                     Expr::Name(Rc::new(("lie").into())),
///                 ),
///             }),
///         }),
///     }),
/// });
/// ```
#[macro_export]
macro_rules! F {
    ('lam $argname:ident . $body:expr) => {
        Rc::new($crate::Expr::Lam {
            argname: Rc::new((stringify! {$argname}).into()),
            body: $body,
        })
    };

    ('app $fn:expr, 'to $arg:expr) => {
        Rc::new($crate::Expr::Call {
            func: $fn,
            arg: $arg,
        })
    };

    ($e:ident) => {
        Rc::new($crate::Expr::Name(Rc::new((stringify! {$e}).into())))
    };

    ($e:expr) => {
        Rc::new($crate::Expr::Put(Rc::new(($e).into())))
    };
}
