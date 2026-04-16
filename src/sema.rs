use crate::ast::*;
use crate::cfg::{BasicBlockRef, CfgBuilder, ControlFlowGraph, Successor};
use crate::lexer::Token;
use crate::lints::*;
use crate::parser::{Cst, Diagnostic, DiagnosticContext, NodeRef, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct SemanticData {
    pub decl_bindings: FxHashMap<NodeRef, NodeRef>,
    pub break_bindings: FxHashMap<NodeRef, NodeRef>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct TypeSet(u16);

impl TypeSet {
    const EMPTY: TypeSet = TypeSet(0);
    const NIL: TypeSet = TypeSet(1);
    const INTEGER: TypeSet = TypeSet(2);
    const FLOAT: TypeSet = TypeSet(4);
    const BOOL: TypeSet = TypeSet(8);
    const STRING: TypeSet = TypeSet(16);
    const FUNCTION: TypeSet = TypeSet(32);
    const TABLE: TypeSet = TypeSet(64);
}
impl std::ops::BitOr for TypeSet {
    type Output = TypeSet;
    fn bitor(self, rhs: Self) -> Self::Output {
        TypeSet(self.0 | rhs.0)
    }
}
impl std::ops::BitOrAssign for TypeSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl std::ops::BitAnd for TypeSet {
    type Output = TypeSet;
    fn bitand(self, rhs: Self) -> Self::Output {
        TypeSet(self.0 & rhs.0)
    }
}
impl std::ops::BitAndAssign for TypeSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[derive(Default, Debug, Clone)]
enum GlobalState {
    #[default]
    Implicit,
    Explicit(NodeRef),
}

#[derive(Debug, Clone)]
struct Local {
    decl: NodeRef,
    used: bool,
    constant: bool,
    ty: TypeSet,
}

#[derive(Debug, Clone)]
struct Global {
    decl: NodeRef,
    constant: bool,
}

#[derive(Debug, Clone)]
enum Variable {
    Local(Local),
    Global(Global),
    Vararg(Varargpar),
}

#[derive(Default, Debug)]
struct Scope<'a> {
    pending_goto: BTreeMap<&'a str, Vec<Gotostat>>,
    label: BTreeMap<&'a str, (Label, bool)>,
    vars: BTreeMap<&'a str, Vec<Variable>>,
    collective_global: Option<Global>,
    skipped: FxHashMap<Gotostat, Vec<NodeRef>>,
    varargs: Option<(Varargpar, bool)>,
    entry: FxHashMap<Label, Stat>,
    func: bool,
}
impl Scope<'_> {
    fn new(func: bool, varargs: Option<(Varargpar, bool)>) -> Self {
        Scope {
            func,
            varargs,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
struct Scopes<'a> {
    scopes: Vec<Scope<'a>>,
}
impl<'a> Scopes<'a> {
    fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    fn push(&mut self, scope: Scope<'a>) {
        self.scopes.push(scope);
    }

    fn pop(
        &mut self,
        diag_ctx: &DiagnosticContext<'a>,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
    ) {
        let top = self.scopes.pop().unwrap();
        for vars in top.vars.values() {
            for Local { decl, used, .. } in vars.iter().filter_map(|var| {
                if let Variable::Local(local) = var {
                    Some(local)
                } else {
                    None
                }
            }) {
                if !used {
                    if let Some(unused_parameter) = diag_ctx.active::<UnusedParameter>()
                        && let Some(par) = Par::cast(cst, *decl)
                        && let Some((name, _)) = par.name(cst)
                        && !name.starts_with('_')
                    {
                        diags.push(unused_parameter.build(cst.span(*decl)));
                    }
                    if let Some(unused_loopvar) = diag_ctx.active::<UnusedLoopvar>()
                        && let Some(var) = Loopvar::cast(cst, *decl)
                        && let Some((name, _)) = var.name(cst)
                        && name != "_"
                        && (!diag_ctx.config.allow_loopvar_unused_hint || !name.starts_with('_'))
                    {
                        diags.push(unused_loopvar.build(cst.span(*decl)));
                    }
                    if let Some(unused_local) = diag_ctx.active::<UnusedLocal>() {
                        if let Some(var) = Attname::cast(cst, *decl) {
                            if let Some((name, _)) = var.name(cst)
                                && name != "_"
                                && (!diag_ctx.config.allow_local_unused_hint
                                    || !name.starts_with('_'))
                            {
                                diags.push(unused_local.build(cst.span(*decl), None));
                            }
                        } else if let Some(func) = Localfuncstat::cast(cst, *decl)
                            && let Some((_, name_span)) = func.name(cst)
                        {
                            diags.push(unused_local.build(name_span, Some(func.span(cst))));
                        }
                    }
                }
            }
        }
        if top.func || self.scopes.is_empty() {
            if let Some(unused_label) = diag_ctx.active::<UnusedLabel>() {
                for (decl, used) in top.label.values() {
                    if !used {
                        diags.push(unused_label.build(decl.span(cst)));
                    }
                }
            }
            for gotostats in top.pending_goto.values() {
                for gotostat in gotostats {
                    if let Some((_, name_span)) = gotostat.name(cst) {
                        diags.push(diag_ctx.undefined_label(name_span));
                    }
                }
            }
            if let Some(unused_vararg) = diag_ctx.active::<UnusedVararg>()
                && let Some((vararg, used)) = top.varargs
                && !used
            {
                diags.push(unused_vararg.build(vararg.span(cst)));
            }
        } else if let Some(scope) = self.scopes.last_mut() {
            for (key, val) in top.pending_goto.iter() {
                scope.pending_goto.entry(key).or_default().extend(val);
            }
        }
    }

    fn insert_local(
        &mut self,
        name: &'a str,
        decl: NodeRef,
        used: bool,
        constant: bool,
        ty: TypeSet,
    ) {
        if let Some(scope) = self.scopes.last_mut() {
            for gotostats in scope.pending_goto.values() {
                for gotostat in gotostats {
                    scope.skipped.entry(*gotostat).or_default().push(decl);
                }
            }
        }
        self.scopes
            .last_mut()
            .unwrap()
            .vars
            .entry(name)
            .or_default()
            .push(Variable::Local(Local {
                decl,
                used,
                constant,
                ty,
            }));
    }

    fn insert_global(&mut self, name: &'a str, decl: NodeRef, constant: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .vars
            .entry(name)
            .or_default()
            .push(Variable::Global(Global { decl, constant }));
    }

    fn insert_vararg(&mut self, name: &'a str, vararg: Varargpar) {
        self.scopes
            .last_mut()
            .unwrap()
            .vars
            .entry(name)
            .or_default()
            .push(Variable::Vararg(vararg));
    }

    fn insert_label(&mut self, name: &'a str, decl: Label, used: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .label
            .insert(name, (decl, used));
    }

    fn insert_entry(&mut self, label: Label, stat: Stat) {
        self.scopes.last_mut().unwrap().entry.insert(label, stat);
    }

    fn get_entry(&mut self, label: Label) -> Option<Stat> {
        self.scopes.last_mut().unwrap().entry.get(&label).copied()
    }

    fn insert_pending_goto(&mut self, name: &'a str, gotostat: Gotostat) {
        self.scopes
            .last_mut()
            .unwrap()
            .pending_goto
            .entry(name)
            .or_default()
            .push(gotostat);
    }

    fn get_var(&mut self, name: &str, lhs: bool) -> Option<(Variable, bool)> {
        for (i, scope) in self.scopes.iter_mut().rev().enumerate() {
            if let Some(var) = scope
                .vars
                .get_mut(name)
                .and_then(|locals| locals.last_mut())
            {
                if let Variable::Local(local) = var {
                    local.used |= !lhs;
                }
                return Some((var.clone(), i == 0));
            }
            if let Some(collective_global) = &scope.collective_global {
                return Some((Variable::Global(collective_global.clone()), i == 0));
            }
        }
        None
    }

    fn get_label(&mut self, name: &str) -> Option<Label> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((decl, used)) = scope.label.get_mut(name) {
                *used = true;
                return Some(*decl);
            }
            if scope.func {
                // labels are not visible across function scopes
                break;
            }
        }
        None
    }

    fn remove_pending_goto(&mut self, name: &str) -> Vec<Gotostat> {
        self.scopes
            .last_mut()
            .and_then(|scope| scope.pending_goto.remove(name))
            .unwrap_or_default()
    }

    fn has_varargs(&mut self) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.func
                && let Some((_, used)) = &mut scope.varargs
            {
                *used = true;
                return true;
            }
            if scope.func {
                // varargs are not visible across function scopes
                return false;
            }
        }
        // chunk is treated like an anonymous variadic function
        true
    }

    fn use_named_vararg(&mut self, vararg: Varargpar) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.func
                && let Some((scope_vararg, used)) = &mut scope.varargs
                && *scope_vararg == vararg
            {
                *used = true;
            }
        }
    }

    fn get_skipped(&self, goto: Gotostat) -> Vec<NodeRef> {
        self.scopes
            .last()
            .and_then(|scope| scope.skipped.get(&goto))
            .map_or_else(Vec::new, |gotostats| gotostats.clone())
    }

    fn get_global_state(&self) -> GlobalState {
        for scope in self.scopes.iter().rev() {
            for vars in scope.vars.values() {
                for var in vars {
                    if let Variable::Global(global) = var {
                        return GlobalState::Explicit(global.decl);
                    }
                }
            }
        }
        GlobalState::Implicit
    }

    fn set_collective_global(&mut self, decl: NodeRef, constant: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.collective_global = Some(Global { decl, constant });
        }
    }
}

pub struct Checks<'a> {
    diag_ctx: DiagnosticContext<'a>,
    scopes: Scopes<'a>,
    loops: Vec<NodeRef>,
}

impl<'a> Checks<'a> {
    pub fn run(diag_ctx: DiagnosticContext<'a>, cst: &Cst<'a>, diags: &mut Vec<Diagnostic<'a>>) {
        let Some(chunk) = Chunk::cast(cst, NodeRef::ROOT) else {
            return;
        };
        let Some(block) = chunk.block(cst) else {
            return;
        };
        let mut sema = SemanticData::default();
        let mut checks = Checks {
            diag_ctx,
            scopes: Scopes::new(),
            loops: Vec::new(),
        };
        checks.check_block(cst, diags, &mut sema, block, None, |_| {});

        let mut cfgs = CfgBuilder::build(cst, &sema, NodeRef::ROOT, block, None);
        checks.check_unreachable(cst, diags, &sema, &mut cfgs);
        checks.check_cyclomatic_complexity(diags, &cfgs);
        checks.check_unconditional_recursion(diags, &cfgs);
    }

    fn check_block(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &mut SemanticData,
        block: Block,
        pars: Option<Pars>,
        add_locals: impl Fn(&mut Scopes<'a>),
    ) {
        self.scopes.push(Scope::new(
            pars.is_some(),
            pars.and_then(|pars| pars.vararg(cst))
                .map(|vararg| (vararg, false)),
        ));
        add_locals(&mut self.scopes);
        if let Some(pars) = pars {
            let mut count = 0;
            for par in pars.pars(cst) {
                count += 1;
                if let Some((name, _)) = par.name(cst) {
                    self.scopes
                        .insert_local(name, par.syntax(), false, false, TypeSet::EMPTY);
                }
            }
            if let Some(vararg) = pars.vararg(cst)
                && let Some((name, _)) = vararg.name(cst)
            {
                self.scopes.insert_vararg(name, vararg);
            }
            if let Some(too_many_parameters) = self.diag_ctx.active::<TooManyParameters>()
                && count >= self.diag_ctx.config.parameter_threshold
            {
                diags.push(too_many_parameters.build(pars.span(cst), count));
            }
        }

        let stats = block.stats(cst).collect::<SmallVec<[_; 128]>>();

        // mark exit labels of block
        let mut last_stat = None;
        for stat in stats.iter().rev() {
            let non_empty_stat = match stat {
                Stat::Label(label) => {
                    if let Some(last_stat) = last_stat {
                        self.scopes.insert_entry(*label, last_stat);
                    }
                    false
                }
                Stat::Emptystat(_) => false,
                Stat::Assignstat(assignstat) => {
                    if let Some(almost_swap) = self.diag_ctx.active::<AlmostSwap>()
                        && let Some(Stat::Assignstat(last_assignstat)) = last_stat
                    {
                        fn rename<'a>(
                            cst: &Cst<'a>,
                            lhs_it: impl Iterator<Item = Exp>,
                            rhs_it: impl Iterator<Item = Exp>,
                        ) -> impl Iterator<Item = (&'a str, &'a str)> {
                            lhs_it.zip(rhs_it).filter_map(|(lhs, rhs)| {
                                if let Exp::Nameexp(lhs_nameexp) = lhs
                                    && let Exp::Nameexp(rhs_nameexp) = rhs
                                    && let Some((lhs_name, _)) = lhs_nameexp.name(cst)
                                    && let Some((rhs_name, _)) = rhs_nameexp.name(cst)
                                {
                                    return Some((lhs_name, rhs_name));
                                }
                                None
                            })
                        }
                        let mut last_renames = rename(
                            cst,
                            last_assignstat.lhs_exps(cst),
                            last_assignstat.rhs_exps(cst),
                        );
                        let mut cur_renames =
                            rename(cst, assignstat.lhs_exps(cst), assignstat.rhs_exps(cst));

                        if let Some((last_to, last_from)) = last_renames.next()
                            && last_renames.count() == 0
                            && let Some((cur_to, cur_from)) = cur_renames.next()
                            && cur_renames.count() == 0
                            && last_to == cur_from
                            && last_from == cur_to
                        {
                            diags.push(almost_swap.build(
                                assignstat.span(cst).start..last_assignstat.span(cst).end,
                                last_from,
                                last_to,
                            ));
                        }
                    }
                    true
                }
                _ => true,
            };
            if non_empty_stat {
                last_stat = Some(*stat);
            }
        }

        let mut last_stat_empty = false;
        for stat in stats {
            if let Some(empty_statement) = self.diag_ctx.active::<EmptyStatement>() {
                let stat_empty = matches!(stat, Stat::Emptystat(_));
                if stat_empty && last_stat_empty {
                    diags.push(empty_statement.build(stat.span(cst)));
                }
                last_stat_empty = stat_empty;
            }
            self.check_statement(cst, diags, sema, stat);
        }
        self.scopes.pop(&self.diag_ctx, cst, diags);
    }

    fn check_rhs_exps(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &mut SemanticData,
        rhs_exps: impl Iterator<Item = Exp>,
    ) -> (bool, bool, Vec<Span>, Vec<TypeSet>) {
        let mut rhs_vararg = false;
        let mut rhs_call = false;
        let mut rhs_spans = vec![];
        let mut rhs_tys = vec![];
        for exp in rhs_exps {
            let exp_span = exp.span(cst);
            rhs_spans.push(exp_span);
            rhs_tys.push(self.check_exp(cst, diags, sema, exp, false, true));
            rhs_vararg |= matches!(exp, Exp::Varargexp(_));
            rhs_call |= matches!(exp, Exp::Callexp(_));
        }
        (rhs_vararg, rhs_call, rhs_spans, rhs_tys)
    }

    fn check_func_lines(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        node_ref: NodeRef,
    ) {
        if let Some(too_many_lines) = self.diag_ctx.active::<TooManyLines>() {
            let span = cst.span(node_ref);
            let lines = cst.source()[span.clone()]
                .bytes()
                .filter(|b| *b == b'\n')
                .count();
            if lines >= self.diag_ctx.config.function_line_threshold {
                diags.push(too_many_lines.build(span, lines));
            }
        }
    }

    fn check_attrib(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        attrib: Option<Attrib>,
        global: bool,
        default_close: bool,
        default_const: bool,
    ) -> (bool, bool) {
        if let Some(attrib) = attrib {
            if let Some((value, span)) = attrib.value(cst)
                && value != "const"
                && (global || value != "close")
            {
                diags.push(self.diag_ctx.unexpected_attribute(span, global));
            }
            (attrib.is_close(cst), attrib.is_const(cst))
        } else {
            (default_close, default_const)
        }
    }

    fn check_local_binding(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        name: &str,
        span: &Span,
    ) {
        if let Some(redefined_local) = self.diag_ctx.active::<RedefinedLocal>()
            && let Some((Variable::Local(Local { decl, .. }), same_scope)) =
                self.scopes.get_var(name, true)
            && same_scope
            && name != "_"
        {
            diags.push(redefined_local.build(
                span.clone(),
                cst.span(decl),
                Funcname::cast(cst, decl).is_some(),
            ))
        }
        if let Some(shadowing_local) = self.diag_ctx.active::<ShadowingLocal>()
            && let Some((Variable::Local(Local { decl, .. }), same_scope)) =
                self.scopes.get_var(name, true)
            && !same_scope
            && name != "_"
        {
            diags.push(shadowing_local.build(span.clone(), cst.span(decl)))
        }
    }

    fn check_statement(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &mut SemanticData,
        stat: Stat,
    ) {
        match stat {
            Stat::Emptystat(_) => {}
            Stat::Expstat(expstat) => {
                if let Some(exp) = expstat.exp(cst) {
                    self.check_exp(cst, diags, sema, exp, false, false);
                }
            }
            Stat::Assignstat(assignstat) => {
                let (rhs_vararg, rhs_call, rhs_spans, _rhs_tys) =
                    self.check_rhs_exps(cst, diags, sema, assignstat.rhs_exps(cst));
                let mut lhs_count = 0;
                let mut lhs_spans = vec![];
                for exp in assignstat.lhs_exps(cst) {
                    let exp_span = exp.span(cst);
                    lhs_spans.push(exp_span);
                    lhs_count += 1;
                    self.check_exp(cst, diags, sema, exp, true, false);
                }
                if let Some(unbalanced_assignment) = self.diag_ctx.active::<UnbalancedAssignment>()
                    && !lhs_spans.is_empty()
                    && !rhs_spans.is_empty()
                    && ((lhs_count > rhs_spans.len() && !rhs_vararg && !rhs_call)
                        || lhs_count < rhs_spans.len())
                    && let Some(equal_span) = assignstat.equal_span(cst)
                {
                    diags.push(unbalanced_assignment.build(&lhs_spans, &rhs_spans, equal_span));
                }
            }
            Stat::Label(label) => {
                let label_span = label.span(cst);
                if let Some(decl) = label
                    .name(cst)
                    .and_then(|(name, _)| self.scopes.get_label(name))
                {
                    diags.push(self.diag_ctx.redefined_label(label_span, decl.span(cst)));
                } else if let Some((name, _)) = label.name(cst) {
                    let pending_gotos = self.scopes.remove_pending_goto(name);
                    if !pending_gotos.is_empty() {
                        for gotostat in pending_gotos {
                            self.scopes.insert_label(name, label, true);
                            sema.decl_bindings.insert(gotostat.syntax(), label.syntax());

                            if let Some(entry) = self.scopes.get_entry(label) {
                                let mut local_spans = vec![];
                                for skipped in self.scopes.get_skipped(gotostat) {
                                    local_spans.push(cst.span(skipped));
                                }
                                if !local_spans.is_empty() {
                                    let goto_span = gotostat.span(cst);
                                    let skipped_span = goto_span.start..label_span.end;
                                    diags.push(self.diag_ctx.goto_skips_local(
                                        goto_span,
                                        skipped_span,
                                        local_spans,
                                        entry.span(cst),
                                    ));
                                }
                            }
                        }
                    } else {
                        self.scopes.insert_label(name, label, false);
                    }
                }
            }
            Stat::Breakstat(breakstat) => {
                if let Some(loopstat) = self.loops.last() {
                    sema.break_bindings.insert(breakstat.syntax(), *loopstat);
                } else {
                    diags.push(self.diag_ctx.break_outside_loop(breakstat.span(cst)));
                }
            }
            Stat::Gotostat(gotostat) => {
                if let Some((name, _)) = gotostat.name(cst) {
                    if let Some(decl) = self.scopes.get_label(name) {
                        sema.decl_bindings.insert(gotostat.syntax(), decl.syntax());
                    } else {
                        self.scopes.insert_pending_goto(name, gotostat);
                    }
                }
            }
            Stat::Dostat(dostat) => {
                if let Some(block) = dostat.block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(dostat.span(cst), block_span));
                    }
                    self.check_block(cst, diags, sema, block, None, |_| {});
                }
            }
            Stat::Whilestat(whilestat) => {
                if let Some(cond) = whilestat.cond(cst) {
                    self.check_exp(cst, diags, sema, cond, false, false);
                }
                self.loops.push(whilestat.syntax());
                if let Some(block) = whilestat.block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(whilestat.span(cst), block_span));
                    }
                    self.check_block(cst, diags, sema, block, None, |_| {});
                }
                self.loops.pop();
            }
            Stat::Repeatstat(repeatstat) => {
                if let Some(cond) = repeatstat.cond(cst) {
                    self.check_exp(cst, diags, sema, cond, false, false);
                }
                self.loops.push(repeatstat.syntax());
                if let Some(block) = repeatstat.block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(repeatstat.span(cst), block_span));
                    }
                    self.check_block(cst, diags, sema, block, None, |_| {});
                }
                self.loops.pop();
            }
            Stat::Ifstat(ifstat) => {
                if let Some(cond) = ifstat.cond(cst) {
                    self.check_exp(cst, diags, sema, cond, false, false);
                }
                if let Some(block) = ifstat.then_block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        let if_span = ifstat.span(cst);
                        diags.push(empty_block.build(if_span.start..block_span.end, block_span));
                    }
                    self.check_block(cst, diags, sema, block, None, |_| {});
                }
                for elif_branch in ifstat.elif_branches(cst) {
                    if let Some(cond) = elif_branch.cond(cst) {
                        self.check_exp(cst, diags, sema, cond, false, false);
                    }
                    if let Some(block) = elif_branch.block(cst) {
                        if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                            && let Some(block_span) = block.empty_block_span(cst)
                        {
                            diags.push(empty_block.build(elif_branch.span(cst), block_span));
                        }
                        self.check_block(cst, diags, sema, block, None, |_| {});
                    }
                }
                if let Some(else_branch) = ifstat.else_branch(cst)
                    && let Some(block) = else_branch.block(cst)
                {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(else_branch.span(cst), block_span));
                    }
                    self.check_block(cst, diags, sema, block, None, |_| {});
                }
            }
            Stat::NumericForstat(forstat) => {
                let (from, to, stride) = forstat.exp_from_to_stride(cst);
                if let Some(from) = from {
                    self.check_exp(cst, diags, sema, from, false, false);
                }
                if let Some(to) = to {
                    self.check_exp(cst, diags, sema, to, false, false);
                }
                if let Some(stride) = stride {
                    self.check_exp(cst, diags, sema, stride, false, false);
                }
                self.loops.push(forstat.syntax());
                if let Some(block) = forstat.block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(forstat.span(cst), block_span));
                    }
                    let const_loop_var = self.diag_ctx.config.lua_minor_version == 5;
                    self.check_block(cst, diags, sema, block, None, |scopes| {
                        if let Some(loopvar) = forstat.loopvar(cst)
                            && let Some((name, _)) = loopvar.name(cst)
                        {
                            scopes.insert_local(
                                name,
                                loopvar.syntax(),
                                false,
                                const_loop_var,
                                TypeSet::EMPTY,
                            );
                        }
                    });
                }
                self.loops.pop();
            }
            Stat::GenericForstat(forstat) => {
                for exp in forstat.exps(cst) {
                    self.check_exp(cst, diags, sema, exp, false, false);
                }
                self.loops.push(forstat.syntax());
                if let Some(block) = forstat.block(cst) {
                    if let Some(empty_block) = self.diag_ctx.active::<EmptyBlock>()
                        && let Some(block_span) = block.empty_block_span(cst)
                    {
                        diags.push(empty_block.build(forstat.span(cst), block_span));
                    }
                    let const_loop_var = self.diag_ctx.config.lua_minor_version == 5;
                    self.check_block(cst, diags, sema, block, None, |scopes| {
                        for (i, loopvar) in forstat.loopvars(cst).enumerate() {
                            if let Some((name, _)) = loopvar.name(cst) {
                                scopes.insert_local(
                                    name,
                                    loopvar.syntax(),
                                    false,
                                    const_loop_var && i == 0,
                                    TypeSet::EMPTY,
                                );
                            }
                        }
                    });
                }
                self.loops.pop();
            }
            Stat::Funcstat(funcstat) => {
                if let Some(block) = funcstat.block(cst)
                    && let Some(funcname) = funcstat.funcname(cst)
                {
                    let qualname = funcname.qualname(cst);
                    let methodname = funcname.methodname(cst);
                    self.check_block(cst, diags, sema, block, funcstat.pars(cst), |scopes| {
                        if methodname.is_some() {
                            scopes.insert_local(
                                "self",
                                funcname.syntax(),
                                true,
                                false,
                                TypeSet::EMPTY,
                            );
                        }
                    });
                    if let Some(qualname) = qualname {
                        let mut names = qualname.names(cst);
                        if let Some((name, name_span)) = names.next() {
                            let unqual_name = names.count() == 0 && methodname.is_none();
                            self.check_name(
                                cst,
                                diags,
                                sema,
                                qualname.syntax(),
                                name,
                                name_span,
                                unqual_name,
                                !unqual_name,
                            );
                        }
                    }
                }
                self.check_func_lines(cst, diags, funcstat.syntax());
            }
            Stat::Localvarstat(localvarstat) => {
                let inits = localvarstat.rhs_exps(cst).collect::<SmallVec<[_; 4]>>();
                let (rhs_vararg, rhs_call, rhs_spans, rhs_tys) =
                    self.check_rhs_exps(cst, diags, sema, inits.iter().copied());

                let (prefix_is_close, prefix_is_const) = self.check_attrib(
                    cst,
                    diags,
                    localvarstat.prefix_attrib(cst),
                    false,
                    false,
                    false,
                );

                let mut lhs_spans = SmallVec::<[_; 4]>::new();
                for (i, attname) in localvarstat.attnames(cst).enumerate() {
                    let (is_close, is_const) = self.check_attrib(
                        cst,
                        diags,
                        attname.attrib(cst),
                        false,
                        prefix_is_close,
                        prefix_is_const,
                    );
                    if let Some((name, span)) = attname.name(cst) {
                        lhs_spans.push(span.clone());
                        self.check_local_binding(cst, diags, name, &span);

                        if let Some(redundant_local) = self.diag_ctx.active::<RedundantLocal>()
                            && let Some(rhs) = inits.get(i)
                            && let Exp::Nameexp(nameexp) = rhs
                            && let Some((rhs_name, rhs_span)) = nameexp.name(cst)
                            && name == rhs_name
                            && let Some(old_decl) = sema.decl_bindings.get(&nameexp.syntax())
                        {
                            diags.push(redundant_local.build(span, rhs_span, cst.span(*old_decl)));
                        }
                        self.scopes.insert_local(
                            name,
                            attname.syntax(),
                            is_close,
                            is_const,
                            *rhs_tys.get(i).unwrap_or(&TypeSet::NIL),
                        );
                    }
                }
                if let Some(unbalanced_initialization) =
                    self.diag_ctx.active::<UnbalancedInitialization>()
                    && !lhs_spans.is_empty()
                    && !rhs_spans.is_empty()
                    && ((lhs_spans.len() > rhs_spans.len() && !rhs_vararg && !rhs_call)
                        || lhs_spans.len() < rhs_spans.len())
                    && let Some(equal_span) = localvarstat.equal_span(cst)
                {
                    diags.push(unbalanced_initialization.build(&lhs_spans, &rhs_spans, equal_span));
                }
            }
            Stat::Localfuncstat(localfuncstat) => {
                if let Some((name, name_span)) = localfuncstat.name(cst) {
                    self.check_local_binding(cst, diags, name, &name_span);
                    self.scopes.insert_local(
                        name,
                        localfuncstat.syntax(),
                        false,
                        false,
                        TypeSet::FUNCTION,
                    );
                }
                if let Some(block) = localfuncstat.block(cst) {
                    self.check_block(cst, diags, sema, block, localfuncstat.pars(cst), |_| {});
                }
                self.check_func_lines(cst, diags, localfuncstat.syntax());
            }
            Stat::Globalvarstat(globalvarstat) => {
                let inits = globalvarstat.rhs_exps(cst).collect::<SmallVec<[_; 4]>>();
                let (rhs_vararg, rhs_call, rhs_spans, _) =
                    self.check_rhs_exps(cst, diags, sema, inits.iter().copied());

                let (_, prefix_is_const) = self.check_attrib(
                    cst,
                    diags,
                    globalvarstat.prefix_attrib(cst),
                    true,
                    false,
                    false,
                );

                let mut lhs_spans = SmallVec::<[_; 4]>::new();
                for attname in globalvarstat.attnames(cst) {
                    let (_, is_const) = self.check_attrib(
                        cst,
                        diags,
                        attname.attrib(cst),
                        true,
                        false,
                        prefix_is_const,
                    );
                    if let Some((name, span)) = attname.name(cst) {
                        lhs_spans.push(span.clone());
                        self.scopes.insert_global(name, attname.syntax(), is_const);
                    }
                }
                if let Some(unbalanced_initialization) =
                    self.diag_ctx.active::<UnbalancedInitialization>()
                    && !lhs_spans.is_empty()
                    && !rhs_spans.is_empty()
                    && ((lhs_spans.len() > rhs_spans.len() && !rhs_vararg && !rhs_call)
                        || lhs_spans.len() < rhs_spans.len())
                    && let Some(equal_span) = globalvarstat.equal_span(cst)
                {
                    diags.push(unbalanced_initialization.build(&lhs_spans, &rhs_spans, equal_span));
                }
            }
            Stat::Globalfuncstat(globalfuncstat) => {
                if let Some((name, name_span)) = globalfuncstat.name(cst) {
                    self.check_local_binding(cst, diags, name, &name_span);
                    self.scopes
                        .insert_global(name, globalfuncstat.syntax(), false);
                }
                if let Some(block) = globalfuncstat.block(cst) {
                    self.check_block(cst, diags, sema, block, globalfuncstat.pars(cst), |_| {});
                }
                self.check_func_lines(cst, diags, globalfuncstat.syntax());
            }
            Stat::CollectiveGlobalvarstat(collective_globalvarstat) => {
                let (_, is_const) = self.check_attrib(
                    cst,
                    diags,
                    collective_globalvarstat.attrib(cst),
                    true,
                    false,
                    false,
                );
                self.scopes
                    .set_collective_global(collective_globalvarstat.syntax(), is_const);
            }
            Stat::Retstat(retstat) => {
                for exp in retstat.exps(cst) {
                    self.check_exp(cst, diags, sema, exp, false, false);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_name(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &mut SemanticData,
        node_ref: NodeRef,
        name: &str,
        name_span: Span,
        lhs: bool,
        sub: bool,
    ) -> TypeSet {
        match self.scopes.get_var(name, lhs) {
            Some((
                Variable::Local(Local {
                    decl, constant, ty, ..
                }),
                _,
            )) => {
                // local definition
                if lhs && constant {
                    diags.push(self.diag_ctx.write_const_variable(
                        name_span.clone(),
                        cst.span(decl),
                        Loopvar::cast(cst, decl).is_some(),
                    ));
                }
                if let Some(used_despite_unused_hint) =
                    self.diag_ctx.active::<UsedDespiteUnusedHint>()
                    && !lhs
                    && (Par::cast(cst, decl).is_some()
                        || (Attname::cast(cst, decl).is_some()
                            && self.diag_ctx.config.allow_local_unused_hint)
                        || (Loopvar::cast(cst, decl).is_some()
                            && self.diag_ctx.config.allow_loopvar_unused_hint))
                    && name != "_G"
                    && name != "_ENV"
                    && name.starts_with('_')
                {
                    diags.push(used_despite_unused_hint.build(name_span, cst.span(decl)))
                }
                sema.decl_bindings.insert(node_ref, decl);
                return ty;
            }
            Some((Variable::Global(Global { decl, constant }), _)) => {
                if lhs && constant {
                    diags.push(self.diag_ctx.write_const_variable(
                        name_span.clone(),
                        cst.span(decl),
                        false,
                    ));
                }
            }
            Some((Variable::Vararg(varargpar), _)) => {
                self.scopes.use_named_vararg(varargpar);
                return TypeSet::TABLE;
            }
            None => {
                // global definition
                match self.scopes.get_global_state() {
                    GlobalState::Explicit(decl) => {
                        if name != "_ENV" {
                            diags.push(self.diag_ctx.undeclared_global(name_span, cst.span(decl)));
                        }
                    }
                    GlobalState::Implicit => {
                        if let Some(lower_case_global) = self.diag_ctx.active::<LowerCaseGlobal>()
                            && name.starts_with(char::is_lowercase)
                            && !self.diag_ctx.config.globals.contains_key(name)
                        {
                            diags.push(lower_case_global.build(name_span, sub, false));
                        }
                    }
                }
            }
        }
        TypeSet::EMPTY
    }

    fn check_exp(
        &mut self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &mut SemanticData,
        exp: Exp,
        lhs: bool,
        sub: bool,
    ) -> TypeSet {
        match exp {
            Exp::Binexp(binexp) => {
                let mut ops = binexp.operands(cst);
                let lhs_exp = ops.next();
                let rhs_exp = ops.next();
                let mut lhs_ty = TypeSet::EMPTY;
                let span = binexp.span(cst);

                if let Some(lhs_exp) = lhs_exp {
                    lhs_ty = self.check_exp(cst, diags, sema, lhs_exp, false, true);
                }
                if let Some(rhs_exp) = rhs_exp {
                    let rhs_ty = self.check_exp(cst, diags, sema, rhs_exp, false, true);

                    if let Some(error_prone_negation) = self.diag_ctx.active::<ErrorProneNegation>()
                    {
                        let is_negation = |exp| {
                            if let Exp::Unaryexp(exp) = exp {
                                return exp.is_negation(cst);
                            }
                            false
                        };
                        if !is_negation(rhs_exp)
                            && let Some(lhs_exp) = lhs_exp
                            && let Some((_, bin_op_span)) = binexp.relational_operator(cst)
                            && let Exp::Unaryexp(operand) = lhs_exp
                            && operand.is_negation(cst)
                            && let Some(un_operand) = operand.operand(cst)
                        {
                            diags.push(error_prone_negation.build(
                                un_operand.span(cst),
                                operand.span(cst),
                                span.clone(),
                                bin_op_span,
                            ));
                        }
                    }

                    if let Some(bool_compare) = self.diag_ctx.active::<BoolCompare>()
                        && let Some((bin_op, _)) = binexp.relational_operator(cst)
                        && let Token::EqualEqual | Token::TildeEqual = bin_op
                        && let Some(lhs_exp) = lhs_exp
                    {
                        match (lhs_exp, rhs_exp) {
                            (Exp::Trueexp(_), _) if rhs_ty == TypeSet::BOOL => {
                                diags.push(bool_compare.build(
                                    span.clone(),
                                    rhs_exp.span(cst),
                                    bin_op == Token::TildeEqual,
                                    matches!(rhs_exp, Exp::Binexp(_)),
                                ));
                            }
                            (Exp::Falseexp(_), _) if rhs_ty == TypeSet::BOOL => {
                                diags.push(bool_compare.build(
                                    span.clone(),
                                    rhs_exp.span(cst),
                                    bin_op == Token::EqualEqual,
                                    matches!(rhs_exp, Exp::Binexp(_)),
                                ));
                            }
                            (_, Exp::Trueexp(_)) if lhs_ty == TypeSet::BOOL => {
                                diags.push(bool_compare.build(
                                    span.clone(),
                                    lhs_exp.span(cst),
                                    bin_op == Token::TildeEqual,
                                    matches!(lhs_exp, Exp::Binexp(_)),
                                ));
                            }
                            (_, Exp::Falseexp(_)) if lhs_ty == TypeSet::BOOL => {
                                diags.push(bool_compare.build(
                                    span.clone(),
                                    lhs_exp.span(cst),
                                    bin_op == Token::EqualEqual,
                                    matches!(lhs_exp, Exp::Binexp(_)),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
            Exp::Unaryexp(unaryexp) => {
                if let Some(operand) = unaryexp.operand(cst) {
                    if let Some(unnecessary_negation) =
                        self.diag_ctx.active::<UnnecessaryNegation>()
                        && let Some((_, operator_span)) = unaryexp.operator(cst)
                        && unaryexp.is_negation(cst)
                        && let Exp::Parenexp(parenexp) = operand
                        && let Some(Exp::Binexp(binexp)) = parenexp.inner(cst)
                        && let Some((bin_op_tok, bin_op_span)) = binexp.relational_operator(cst)
                    {
                        diags.push(unnecessary_negation.build(
                            operator_span,
                            bin_op_tok,
                            bin_op_span,
                            binexp.span(cst),
                            parenexp.span(cst),
                        ));
                    }
                    self.check_exp(cst, diags, sema, operand, false, true);
                }
            }
            Exp::Nameexp(nameexp) => {
                if let Some((name, name_span)) = nameexp.name(cst) {
                    return self.check_name(
                        cst,
                        diags,
                        sema,
                        nameexp.syntax(),
                        name,
                        name_span,
                        lhs,
                        sub,
                    );
                }
            }
            Exp::Parenexp(parenexp) => {
                if let Some(inner) = parenexp.inner(cst) {
                    let res = self.check_exp(cst, diags, sema, inner, lhs, true);
                    if let Some(redundant_parentheses) =
                        self.diag_ctx.active::<RedundantParentheses>()
                        && let Exp::Parenexp(inner_parenexp) = inner
                    {
                        diags.push(
                            redundant_parentheses
                                .build(parenexp.span(cst), inner_parenexp.span(cst)),
                        );
                    }
                    return res;
                }
            }
            Exp::Fieldexp(fieldexp) => {
                if let Some(base) = fieldexp.base(cst) {
                    self.check_exp(cst, diags, sema, base, false, true);
                }
            }
            Exp::Indexexp(indexexp) => {
                let (base, index) = indexexp.base_and_index(cst);
                if let Some(base) = base {
                    self.check_exp(cst, diags, sema, base, false, true);
                }
                if let Some(index) = index {
                    self.check_exp(cst, diags, sema, index, false, true);
                }
            }
            Exp::Callexp(callexp) => {
                if let Some(base) = callexp.base(cst) {
                    self.check_exp(cst, diags, sema, base, false, true);
                    if let Some(args) = callexp.args(cst) {
                        if let Some(next_line_args) = self.diag_ctx.active::<NextLineArgs>()
                            && args.in_next_line(cst)
                        {
                            diags.push(next_line_args.build(args.span(cst), base.span(cst)));
                        }
                        for argexp in args.expressions(cst) {
                            self.check_exp(cst, diags, sema, argexp, false, true);
                        }
                    }
                }
            }
            Exp::Functiondef(functiondef) => {
                if let Some(block) = functiondef.block(cst) {
                    self.check_block(cst, diags, sema, block, functiondef.pars(cst), |_| {});
                }
                self.check_func_lines(cst, diags, functiondef.syntax());
            }
            Exp::Tableconstructor(tableconstructor) => {
                for field in tableconstructor.fields(cst) {
                    match field {
                        Field::KeyvalField(keyval_field) => {
                            let (key, val) = keyval_field.key_val(cst);
                            if let Some(key) = key {
                                self.check_exp(cst, diags, sema, key, false, true);
                            }
                            if let Some(val) = val {
                                self.check_exp(cst, diags, sema, val, false, true);
                            }
                        }
                        Field::NamedField(named_field) => {
                            let val = named_field.val(cst);
                            if let Some(val) = val {
                                self.check_exp(cst, diags, sema, val, false, true);
                            }
                        }
                        Field::CountedField(counted_field) => {
                            let val = counted_field.val(cst);
                            if let Some(val) = val {
                                self.check_exp(cst, diags, sema, val, false, true);
                            }
                        }
                    }
                }
                return TypeSet::TABLE;
            }
            Exp::Varargexp(varargexp) => {
                if !self.scopes.has_varargs() {
                    diags.push(self.diag_ctx.invalid_vararg(varargexp.span(cst)));
                }
            }
            Exp::Decintexp(numexp) => {
                fn f64_int_str(n: f64) -> String {
                    format!("{n:.17}").split_once('.').unwrap().0.to_string()
                }
                if let Some((num_str, num_span)) = numexp.val(cst) {
                    if let Some(octal_confusion) = self.diag_ctx.active::<OctalConfusion>()
                        && num_str.len() > 1
                        && num_str.starts_with('0')
                    {
                        diags.push(octal_confusion.build(num_span.clone()));
                    }
                    match num_str.parse::<i64>() {
                        Ok(_) => {}
                        Err(err) => match err.kind() {
                            std::num::IntErrorKind::PosOverflow
                            | std::num::IntErrorKind::NegOverflow => {
                                if let Ok(num_f64) = str::parse::<f64>(num_str) {
                                    if let Some(rounds_to_inf) =
                                        self.diag_ctx.active::<RoundsToInf>()
                                        && num_f64.is_infinite()
                                    {
                                        diags.push(rounds_to_inf.build(num_span));
                                    } else if let Some(rounds_int_part) =
                                        self.diag_ctx.active::<RoundsIntPart>()
                                        && num_str != f64_int_str(num_f64)
                                    {
                                        diags.push(rounds_int_part.build(num_span, num_f64));
                                    }
                                    return TypeSet::FLOAT;
                                }
                            }
                            _ => unreachable!(),
                        },
                    }
                }
                return TypeSet::INTEGER;
            }
            Exp::Hexintexp(numexp) => {
                if let Some((num_str, num_span)) = numexp.val(cst) {
                    let num_str_no_prefix = &num_str[2..];
                    match i64::from_str_radix(num_str_no_prefix, 16) {
                        Ok(_) => {}
                        Err(err) => match err.kind() {
                            std::num::IntErrorKind::PosOverflow
                            | std::num::IntErrorKind::NegOverflow => {
                                if let Some(hex_int_overflow) =
                                    self.diag_ctx.active::<HexIntOverflow>()
                                {
                                    let digit_offset = num_str_no_prefix.len().saturating_sub(16);
                                    let unsigned_val =
                                        u64::from_str_radix(&num_str_no_prefix[digit_offset..], 16)
                                            .unwrap();
                                    let actual_val = unsigned_val.cast_signed();
                                    diags.push(hex_int_overflow.build(num_span, actual_val));
                                }
                            }
                            _ => unreachable!(),
                        },
                    }
                }
                return TypeSet::INTEGER;
            }
            Exp::Decfloatexp(numexp) => {
                fn f64_int_str(n: f64) -> String {
                    format!("{n:.17}").split_once('.').unwrap().0.to_string()
                }
                if let Some((num_str, num_span)) = numexp.val(cst)
                    && let Ok(num_f64) = str::parse::<f64>(num_str)
                {
                    if let Some(rounds_to_inf) = self.diag_ctx.active::<RoundsToInf>()
                        && num_f64.is_infinite()
                    {
                        diags.push(rounds_to_inf.build(num_span));
                    } else if let Some(rounds_int_part) = self.diag_ctx.active::<RoundsIntPart>()
                        && !num_str.contains(['e', 'E'])
                        && let Some(dot_index) = num_str.find('.')
                        && (!num_str[..dot_index].is_empty()
                            && num_str[..dot_index] != f64_int_str(num_f64))
                    {
                        diags.push(rounds_int_part.build(num_span, num_f64));
                    } else if let Some(approx_pi) = self.diag_ctx.active::<ApproxPi>()
                        && (num_f64 - std::f64::consts::PI).abs() < 0.001
                    {
                        diags.push(approx_pi.build(num_span.clone()));
                    }
                }
                return TypeSet::FLOAT;
            }
            Exp::Hexfloatexp(numexp) => {
                if let Some((num_str, num_span)) = numexp.val(cst) {
                    let val = if num_str.contains(['p', 'P']) {
                        hexf_parse::parse_hexf64(num_str, false)
                    } else {
                        let num_str_with_exp = num_str.to_string() + "p0";
                        hexf_parse::parse_hexf64(&num_str_with_exp, false)
                    };
                    match val {
                        Ok(_) => {
                            return TypeSet::FLOAT;
                        }
                        Err(_) => {
                            if let Some(inexact_hex_float) =
                                self.diag_ctx.active::<InexactHexFloat>()
                            {
                                diags.push(inexact_hex_float.build(num_span));
                            }
                        }
                    }
                }
                return TypeSet::FLOAT;
            }
            Exp::Nilexp(_) => return TypeSet::NIL,
            Exp::Trueexp(_) | Exp::Falseexp(_) => return TypeSet::BOOL,
            Exp::Stringexp(_) => return TypeSet::STRING,
        }
        TypeSet::EMPTY
    }

    fn check_unreachable(
        &self,
        cst: &Cst<'a>,
        diags: &mut Vec<Diagnostic<'a>>,
        sema: &SemanticData,
        cfgs: &mut [ControlFlowGraph],
    ) {
        if let Some(unreachable_code) = self.diag_ctx.active::<UnreachableCode>() {
            for cfg in cfgs {
                let mut todo = vec![BasicBlockRef::ENTRY];
                let mut visited = FxHashSet::default();
                while let Some(bb_ref) = todo.pop() {
                    if !visited.insert(bb_ref) {
                        continue;
                    }
                    let bb = cfg.bb_mut(bb_ref);
                    bb.reachable = true;
                    match bb.successor {
                        Successor::None => {}
                        Successor::Interproc(ref succs) => {
                            for succ in succs {
                                todo.push(*succ);
                            }
                        }
                        Successor::Uncond(succ) => {
                            todo.push(succ);
                        }
                        Successor::Cond {
                            exp,
                            false_bb,
                            true_bb,
                        } => {
                            let cond_val = exp.and_then(|cond| self.const_eval_bool(sema, cond));
                            match cond_val {
                                Some(true) => todo.push(true_bb),
                                Some(false) => todo.push(false_bb),
                                None => {
                                    todo.push(true_bb);
                                    todo.push(false_bb);
                                }
                            }
                        }
                    }
                }

                let mut unreachable_bb_spans = BTreeMap::<usize, Span>::new();
                for bb in cfg.bbs.iter() {
                    if !bb.reachable && bb.span.start != bb.span.end {
                        match unreachable_bb_spans
                            .range_mut(0..=bb.span.start)
                            .next_back()
                        {
                            Some((start, span))
                                if *start == bb.span.start || span.end >= bb.span.start =>
                            {
                                span.end = span.end.max(bb.span.end);
                            }
                            _ => {
                                // ignore unreachable blocks which only contain a semicolon
                                if &cst.source()[bb.span.clone()] != ";" {
                                    unreachable_bb_spans.insert(bb.span.start, bb.span.clone());
                                }
                            }
                        }
                    }
                }
                if !unreachable_bb_spans.is_empty() {
                    diags.push(unreachable_code.build(unreachable_bb_spans));
                }
            }
        }
    }

    fn const_eval_bool(&self, _sema: &SemanticData, _exp: Exp) -> Option<bool> {
        None // TODO
    }

    fn check_cyclomatic_complexity(
        &self,
        diags: &mut Vec<Diagnostic<'a>>,
        cfgs: &[ControlFlowGraph],
    ) {
        if let Some(cyclomatic_complexity) = self.diag_ctx.active::<CyclomaticComplexity>() {
            for cfg in cfgs {
                if let Some(span) = &cfg.span {
                    let cc = cfg.edges + cfg.terminators + 1 - cfg.bbs.len();
                    if cc >= self.diag_ctx.config.cyclomatic_complexity_threshold {
                        diags.push(cyclomatic_complexity.build(span.clone(), cc));
                    }
                }
            }
        }
    }

    fn check_unconditional_recursion(
        &self,
        diags: &mut Vec<Diagnostic<'a>>,
        cfgs: &[ControlFlowGraph],
    ) {
        if let Some(uncoditional_recursion) = self.diag_ctx.active::<UnconditionalRecursion>() {
            'cfg: for cfg in cfgs {
                let mut rec_exits = vec![];
                let mut term_exits = vec![];
                for bb in cfg.bbs.iter() {
                    if bb.reachable {
                        if bb.returning.is_some() || bb.yielding.is_some() {
                            continue 'cfg;
                        }
                        if let Some(ref recursive) = bb.recursive {
                            rec_exits.push(recursive.clone());
                        } else if let Some(ref terminate) = bb.terminate {
                            term_exits.push(terminate.clone());
                        }
                    }
                }
                if !rec_exits.is_empty()
                    && let Some(span) = &cfg.span
                {
                    diags.push(uncoditional_recursion.build(span.clone(), &rec_exits, &term_exits));
                }
            }
        }
    }
}
