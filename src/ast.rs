use crate::lexer::Token;
use crate::parser::{Cst, CstChildren, Node, NodeRef, Rule, Span};

#[allow(dead_code)]
pub trait AstNode {
    fn cast(cst: &Cst<'_>, syntax: NodeRef) -> Option<Self>
    where
        Self: Sized;

    fn syntax(&self) -> NodeRef;

    fn span(&self, cst: &Cst<'_>) -> Span {
        cst.span(self.syntax())
    }
}

macro_rules! ast_node {
    ($node_name:ident) => {
        #[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Ord, PartialOrd)]
        pub struct $node_name {
            syntax: NodeRef,
        }
        impl AstNode for $node_name {
            fn cast(cst: &Cst<'_>, syntax: NodeRef) -> Option<Self> {
                match cst.get(syntax) {
                    Node::Rule(Rule::$node_name, _) => Some(Self { syntax }),
                    _ => None,
                }
            }
            fn syntax(&self) -> NodeRef {
                self.syntax
            }
        }
    };
    ($node_name:ident, ($($node_names:ident),+)) => {
        #[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Ord, PartialOrd)]
        #[allow(clippy::enum_variant_names)]
        pub enum $node_name {
            $($node_names($node_names),)*
        }
        impl AstNode for $node_name {
            fn cast(cst: &Cst<'_>, syntax: NodeRef) -> Option<Self> {
                $(
                if let Some(node) = $node_names::cast(cst, syntax) {
                    return Some(Self::$node_names(node));
                }
                )*
                return None;
            }
            fn syntax(&self) -> NodeRef {
                match self {
                    $(Self::$node_names(node) => node.syntax(),)*
                }
            }
        }
    }
}

ast_node!(Chunk);
ast_node!(Block);
ast_node!(Emptystat);
ast_node!(Expstat);
ast_node!(Assignstat);
ast_node!(Label);
ast_node!(Breakstat);
ast_node!(Gotostat);
ast_node!(Dostat);
ast_node!(Whilestat);
ast_node!(Repeatstat);
ast_node!(Ifstat);
ast_node!(ElifBranch);
ast_node!(ElseBranch);
ast_node!(NumericForstat);
ast_node!(GenericForstat);
ast_node!(Loopvar);
ast_node!(Funcstat);
ast_node!(Funcname);
ast_node!(Pars);
ast_node!(Par);
ast_node!(Varargpar);
ast_node!(Qualname);
ast_node!(Localvarstat);
ast_node!(Localfuncstat);
ast_node!(Globalvarstat);
ast_node!(Globalfuncstat);
ast_node!(CollectiveGlobalvarstat);
ast_node!(Attname);
ast_node!(Attrib);
ast_node!(Retstat);
ast_node!(
    Stat,
    (
        Emptystat,
        Expstat,
        Assignstat,
        Label,
        Breakstat,
        Gotostat,
        Dostat,
        Whilestat,
        Repeatstat,
        Ifstat,
        NumericForstat,
        GenericForstat,
        Funcstat,
        Localvarstat,
        Localfuncstat,
        Globalvarstat,
        Globalfuncstat,
        CollectiveGlobalvarstat,
        Retstat
    )
);
ast_node!(Binexp);
ast_node!(Unaryexp);
ast_node!(Nameexp);
ast_node!(Parenexp);
ast_node!(Fieldexp);
ast_node!(Indexexp);
ast_node!(Callexp);
ast_node!(Args, (ArgsExplist, ArgsSingle));
ast_node!(ArgsExplist);
ast_node!(ArgsSingle);
ast_node!(Functiondef);
ast_node!(Tableconstructor);
ast_node!(Field, (KeyvalField, NamedField, CountedField));
ast_node!(KeyvalField);
ast_node!(NamedField);
ast_node!(CountedField);
ast_node!(Varargexp);
ast_node!(Nilexp);
ast_node!(Trueexp);
ast_node!(Falseexp);
ast_node!(Stringexp);
ast_node!(Decintexp);
ast_node!(Hexintexp);
ast_node!(Decfloatexp);
ast_node!(Hexfloatexp);
ast_node!(
    Exp,
    (
        Binexp,
        Unaryexp,
        Nameexp,
        Parenexp,
        Fieldexp,
        Indexexp,
        Callexp,
        Functiondef,
        Tableconstructor,
        Varargexp,
        Nilexp,
        Trueexp,
        Falseexp,
        Stringexp,
        Decintexp,
        Hexintexp,
        Decfloatexp,
        Hexfloatexp
    )
);

impl<'a> Cst<'a> {
    fn child_node<T: AstNode>(&self, syntax: NodeRef) -> Option<T> {
        self.children(syntax).find_map(|c| T::cast(self, c))
    }
    fn child_node_iter<T: AstNode>(
        &self,
        syntax: NodeRef,
    ) -> std::iter::FilterMap<CstChildren<'_>, impl FnMut(NodeRef) -> Option<T> + '_> {
        self.children(syntax).filter_map(|c| T::cast(self, c))
    }
    fn any_child_token(&self, syntax: NodeRef, tokens: &[Token]) -> Option<(Token, Span)> {
        self.children(syntax).find_map(|c| {
            if let Node::Token(tok, _) = self.get(c) {
                if tokens.contains(&tok) {
                    Some((tok, self.span(c)))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
    fn child_token(&self, syntax: NodeRef, token: Token) -> Option<(&'a str, Span)> {
        self.children(syntax)
            .find_map(|c| self.match_token(c, token))
    }
    fn child_token_iter(
        &'a self,
        syntax: NodeRef,
        token: Token,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<(&'a str, Span)> + 'a>
    {
        self.children(syntax)
            .filter_map(move |c| self.match_token(c, token))
    }
}

impl Chunk {
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Block {
    pub fn stats<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Stat> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn retstat(&self, cst: &Cst<'_>) -> Option<Retstat> {
        cst.child_node(self.syntax)
    }
    fn contains_comment(&self, cst: &Cst<'_>) -> bool {
        // rule node never starts with skipped token so search backward for comment
        for i in 1.. {
            let node_ref = NodeRef(self.syntax.0.saturating_sub(i));
            if cst.match_token(node_ref, Token::Comment).is_some() {
                return true;
            }
            if cst.match_token(node_ref, Token::Whitespace).is_none() || node_ref.0 == 0 {
                return false;
            }
        }
        false
    }
    pub fn empty_block_span(&self, cst: &Cst<'_>) -> Option<Span> {
        if self.stats(cst).count() == 0 && !self.contains_comment(cst) {
            let node_ref = NodeRef(self.syntax().0.saturating_sub(1));
            if node_ref.0 == 0 {
                return None;
            }
            if cst.match_token(node_ref, Token::Whitespace).is_some() {
                return Some(cst.span(node_ref));
            }
        }
        None
    }
}

impl Expstat {
    pub fn exp(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
}

impl Ifstat {
    pub fn cond(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn then_block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
    pub fn elif_branches<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<ElifBranch> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn else_branch(&self, cst: &Cst<'_>) -> Option<ElseBranch> {
        cst.child_node(self.syntax)
    }
}

impl ElifBranch {
    pub fn cond(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl ElseBranch {
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl NumericForstat {
    pub fn loopvar(&self, cst: &Cst<'_>) -> Option<Loopvar> {
        cst.child_node(self.syntax)
    }
    pub fn exp_from_to_stride(&self, cst: &Cst<'_>) -> (Option<Exp>, Option<Exp>, Option<Exp>) {
        let mut iter = cst.children(self.syntax).filter_map(|c| Exp::cast(cst, c));
        (iter.next(), iter.next(), iter.next())
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl GenericForstat {
    pub fn loopvars<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Loopvar> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.children(self.syntax)
            .find_map(|c| {
                if cst.match_rule(c, Rule::Explist) {
                    Some(cst.children(c))
                } else {
                    None
                }
            })
            .unwrap_or(CstChildren::default())
            .filter_map(|c| Exp::cast(cst, c))
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Loopvar {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}

impl Whilestat {
    pub fn cond(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Repeatstat {
    pub fn cond(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Dostat {
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Assignstat {
    pub fn lhs_exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn equal_span(&self, cst: &Cst<'_>) -> Option<Span> {
        cst.child_token(self.syntax, Token::Equal)
            .map(|(_, span)| span)
    }
    pub fn rhs_exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.children(self.syntax)
            .find_map(|c| {
                if cst.match_rule(c, Rule::Explist) {
                    Some(cst.children(c))
                } else {
                    None
                }
            })
            .unwrap_or(CstChildren::default())
            .filter_map(|c| Exp::cast(cst, c))
    }
}

impl Funcstat {
    pub fn funcname(&self, cst: &Cst<'_>) -> Option<Funcname> {
        cst.child_node(self.syntax)
    }
    pub fn pars(&self, cst: &Cst<'_>) -> Option<Pars> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}
impl Funcname {
    pub fn qualname(&self, cst: &Cst<'_>) -> Option<Qualname> {
        cst.child_node(self.syntax)
    }
    pub fn methodname<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}
impl Pars {
    pub fn pars<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Par> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn vararg(&self, cst: &Cst<'_>) -> Option<Varargpar> {
        cst.child_node(self.syntax)
    }
}
impl Par {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}
impl Varargpar {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}
impl Qualname {
    pub fn names<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<(&'a str, Span)> + 'a>
    {
        cst.child_token_iter(self.syntax, Token::Name)
    }
}

impl Localvarstat {
    pub fn attnames<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Attname> + 'a> {
        cst.child_node_iter(self.syntax())
    }
    pub fn prefix_attrib(&self, cst: &Cst<'_>) -> Option<Attrib> {
        cst.child_node(self.syntax())
    }
    pub fn equal_span(&self, cst: &Cst<'_>) -> Option<Span> {
        cst.child_token(self.syntax(), Token::Equal)
            .map(|(_, span)| span)
    }
    pub fn rhs_exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.children(self.syntax())
            .find_map(|c| {
                if cst.match_rule(c, Rule::Explist) {
                    Some(cst.children(c))
                } else {
                    None
                }
            })
            .unwrap_or(CstChildren::default())
            .filter_map(|c| Exp::cast(cst, c))
    }
}

impl Label {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}
impl Gotostat {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}

impl Localfuncstat {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
    pub fn pars(&self, cst: &Cst<'_>) -> Option<Pars> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl Globalvarstat {
    pub fn attnames<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Attname> + 'a> {
        cst.child_node_iter(self.syntax())
    }
    pub fn prefix_attrib(&self, cst: &Cst<'_>) -> Option<Attrib> {
        cst.child_node(self.syntax())
    }
    pub fn equal_span(&self, cst: &Cst<'_>) -> Option<Span> {
        cst.child_token(self.syntax(), Token::Equal)
            .map(|(_, span)| span)
    }
    pub fn rhs_exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.children(self.syntax())
            .find_map(|c| {
                if cst.match_rule(c, Rule::Explist) {
                    Some(cst.children(c))
                } else {
                    None
                }
            })
            .unwrap_or(CstChildren::default())
            .filter_map(|c| Exp::cast(cst, c))
    }
}

impl Globalfuncstat {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
    pub fn pars(&self, cst: &Cst<'_>) -> Option<Pars> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}

impl CollectiveGlobalvarstat {
    pub fn attrib(&self, cst: &Cst<'_>) -> Option<Attrib> {
        cst.child_node(self.syntax)
    }
}

impl Retstat {
    pub fn exps<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.children(self.syntax)
            .find_map(|c| {
                if cst.match_rule(c, Rule::Explist) {
                    Some(cst.children(c))
                } else {
                    None
                }
            })
            .unwrap_or(CstChildren::default())
            .filter_map(|c| Exp::cast(cst, c))
    }
}

impl Attname {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
    pub fn attrib(&self, cst: &Cst<'_>) -> Option<Attrib> {
        cst.child_node(self.syntax)
    }
}
impl Attrib {
    pub fn is_close(&self, cst: &Cst) -> bool {
        self.value(cst).is_some_and(|(value, _)| value == "close")
    }
    pub fn is_const(&self, cst: &Cst) -> bool {
        self.value(cst).is_some_and(|(value, _)| value == "const")
    }
    pub fn value<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}

impl Binexp {
    pub fn operands<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        cst.child_node_iter(self.syntax)
    }
    pub fn relational_operator(&self, cst: &Cst<'_>) -> Option<(Token, Span)> {
        cst.any_child_token(
            self.syntax,
            &[
                Token::Less,
                Token::Greater,
                Token::LessEqual,
                Token::GreaterEqual,
                Token::TildeEqual,
                Token::EqualEqual,
            ],
        )
    }
    pub fn is_and(&self, cst: &Cst<'_>) -> bool {
        cst.child_token(self.syntax, Token::And).is_some()
    }
    pub fn is_or(&self, cst: &Cst<'_>) -> bool {
        cst.child_token(self.syntax, Token::Or).is_some()
    }
    pub fn is_concat(&self, cst: &Cst<'_>) -> bool {
        cst.child_token(self.syntax, Token::Dot2).is_some()
    }
}

impl Unaryexp {
    pub fn operand(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn operator(&self, cst: &Cst<'_>) -> Option<(Token, Span)> {
        cst.any_child_token(
            self.syntax,
            &[Token::Not, Token::Minus, Token::Hash, Token::Tilde],
        )
    }
    pub fn is_negation(&self, cst: &Cst<'_>) -> bool {
        cst.child_token(self.syntax, Token::Not).is_some()
    }
}

impl Nameexp {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}

impl Parenexp {
    pub fn inner(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
}

impl Fieldexp {
    pub fn base(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn field<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
}

impl Indexexp {
    pub fn base_and_index(&self, cst: &Cst<'_>) -> (Option<Exp>, Option<Exp>) {
        let mut iter = cst.children(self.syntax).filter_map(|c| Exp::cast(cst, c));
        (iter.next(), iter.next())
    }
}

impl Callexp {
    pub fn base(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
    pub fn args(&self, cst: &Cst<'_>) -> Option<Args> {
        cst.child_node(self.syntax)
    }
}

impl Args {
    pub fn expressions<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Exp> + 'a> {
        match self {
            Args::ArgsExplist(args_explist) => cst.child_node_iter(args_explist.syntax),
            Args::ArgsSingle(arg_single) => cst.child_node_iter(arg_single.syntax),
        }
    }
    pub fn in_next_line(&self, cst: &Cst<'_>) -> bool {
        for i in 1.. {
            let node_ref = NodeRef(self.syntax().0.saturating_sub(i));
            if let Some((ws, _)) = cst.match_token(node_ref, Token::Whitespace) {
                return ws.contains('\n');
            }
            if cst.match_token(node_ref, Token::Comment).is_none() || node_ref.0 == 0 {
                return false;
            }
        }
        false
    }
}
impl Functiondef {
    pub fn pars(&self, cst: &Cst<'_>) -> Option<Pars> {
        cst.child_node(self.syntax)
    }
    pub fn block(&self, cst: &Cst<'_>) -> Option<Block> {
        cst.child_node(self.syntax)
    }
}
impl Tableconstructor {
    pub fn fields<'a>(
        &self,
        cst: &'a Cst<'_>,
    ) -> std::iter::FilterMap<CstChildren<'a>, impl FnMut(NodeRef) -> Option<Field> + 'a> {
        cst.child_node_iter(self.syntax)
    }
}
impl KeyvalField {
    pub fn key_val(&self, cst: &Cst<'_>) -> (Option<Exp>, Option<Exp>) {
        let mut iter = cst.children(self.syntax).filter_map(|c| Exp::cast(cst, c));
        (iter.next(), iter.next())
    }
}
impl NamedField {
    pub fn name<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::Name)
    }
    pub fn val(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
}
impl CountedField {
    pub fn val(&self, cst: &Cst<'_>) -> Option<Exp> {
        cst.child_node(self.syntax)
    }
}

impl Decintexp {
    pub fn val<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::DecIntNumeral)
    }
}

impl Hexintexp {
    pub fn val<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::HexIntNumeral)
    }
}

impl Decfloatexp {
    pub fn val<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::DecFloatNumeral)
    }
}

impl Hexfloatexp {
    pub fn val<'a>(&self, cst: &Cst<'a>) -> Option<(&'a str, Span)> {
        cst.child_token(self.syntax, Token::HexFloatNumeral)
    }
}
