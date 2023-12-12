use crate::builders::parenthesize_if_expands;
use ruff_formatter::write;
use ruff_python_ast::{Expr, StmtAnnAssign};

use crate::comments::{SourceComment, SuppressionKind};
use crate::expression::parentheses::Parentheses;
use crate::expression::{has_own_parentheses, has_parentheses};
use crate::prelude::*;
use crate::preview::{
    is_parenthesize_long_type_hints_enabled,
    is_prefer_splitting_right_hand_side_of_assignments_enabled,
};
use crate::statement::stmt_assign::{
    AnyAssignmentOperator, AnyBeforeOperator, FormatStatementsLastExpression,
};
use crate::statement::trailing_semicolon;

#[derive(Default)]
pub struct FormatStmtAnnAssign;

impl FormatNodeRule<StmtAnnAssign> for FormatStmtAnnAssign {
    fn fmt_fields(&self, item: &StmtAnnAssign, f: &mut PyFormatter) -> FormatResult<()> {
        let StmtAnnAssign {
            range: _,
            target,
            annotation,
            value,
            simple: _,
        } = item;

        write!(f, [target.format(), token(":"), space()])?;

        if let Some(value) = value {
            if is_prefer_splitting_right_hand_side_of_assignments_enabled(f.context())
                && (has_parentheses(annotation, f.context()).is_some()
                    || (is_parenthesize_long_type_hints_enabled(f.context()))
                        && should_parenthesize_annotation(annotation, f.context()))
            {
                FormatStatementsLastExpression::RightToLeft {
                    before_operator: AnyBeforeOperator::Expression(annotation),
                    operator: AnyAssignmentOperator::Assign,
                    value,
                    statement: item.into(),
                }
                .fmt(f)?;
            } else {
                write!(
                    f,
                    [
                        annotation.format(),
                        space(),
                        token("="),
                        space(),
                        FormatStatementsLastExpression::left_to_right(value, item)
                    ]
                )?;
            }
        } else {
            FormatAnnotation::new(annotation).fmt(f)?;
        }

        if f.options().source_type().is_ipynb()
            && f.context().node_level().is_last_top_level_statement()
            && target.is_name_expr()
            && trailing_semicolon(item.into(), f.context().source()).is_some()
        {
            token(";").fmt(f)?;
        }

        Ok(())
    }

    fn is_suppressed(
        &self,
        trailing_comments: &[SourceComment],
        context: &PyFormatContext,
    ) -> bool {
        SuppressionKind::has_skip_comment(trailing_comments, context.source())
    }
}

pub(crate) struct FormatAnnotation<'a> {
    annotation: &'a Expr,
}

impl<'a> FormatAnnotation<'a> {
    pub(crate) fn new(annotation: &'a Expr) -> Self {
        Self { annotation }
    }
}

impl Format<PyFormatContext<'_>> for FormatAnnotation<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        if is_parenthesize_long_type_hints_enabled(f.context()) {
            let comments = f.context().comments();
            if comments.has_leading(self.annotation) || comments.has_trailing(self.annotation) {
                self.annotation
                    .format()
                    .with_options(Parentheses::Always)
                    .fmt(f)
            } else if should_parenthesize_annotation(self.annotation, f.context()) {
                parenthesize_if_expands(&self.annotation.format().with_options(Parentheses::Never))
                    .fmt(f)
            } else {
                self.annotation
                    .format()
                    .with_options(Parentheses::Never)
                    .fmt(f)
            }
        } else {
            self.annotation.format().fmt(f)
        }
    }
}

/// Returns `true` if an annotation should be parenthesized if it splits over multiple lines.
pub(crate) fn should_parenthesize_annotation(
    annotation: &Expr,
    context: &PyFormatContext<'_>,
) -> bool {
    !matches!(annotation, Expr::Name(_)) && has_own_parentheses(annotation, context).is_none()
}
