//! Linear question, review, and completion state with application-owned answers.

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div,
};

use crate::{ObservedElement, TestSupportExt as _};

/// The current part of a linear questionnaire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionnaireStage {
    Question(usize),
    Review,
    Complete,
}

/// A small, presentation-independent questionnaire controller.
///
/// `A` is the application's answer type, which may be an enum for different
/// kinds of question. Editors, prompts, validation, and persistence belong to the
/// application. There is no schema interpreter or form field registry. Retain
/// this value in the application's view, and notify that view after navigation.
/// Answer values survive Back, review, completion, and editing a previous step.
pub struct QuestionnaireState<A> {
    answers: Vec<Option<A>>,
    stage: QuestionnaireStage,
    error: Option<SharedString>,
}

impl<A> QuestionnaireState<A> {
    /// Creates a fixed linear flow. An empty flow starts at Review.
    pub fn new(question_count: usize) -> Self {
        Self {
            answers: (0..question_count).map(|_| None).collect(),
            stage: if question_count == 0 {
                QuestionnaireStage::Review
            } else {
                QuestionnaireStage::Question(0)
            },
            error: None,
        }
    }

    pub fn stage(&self) -> QuestionnaireStage {
        self.stage
    }
    pub fn question_count(&self) -> usize {
        self.answers.len()
    }
    pub fn answer(&self, index: usize) -> Option<&A> {
        self.answers.get(index).and_then(Option::as_ref)
    }
    pub fn answers(&self) -> &[Option<A>] {
        &self.answers
    }
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// Retains the submitted answer, including when validation rejects it.
    /// Advances only on the application's successful validation result.
    /// An error stays on its question; submit again to replace it.
    pub fn submit_answer(&mut self, answer: A, validation: Result<(), SharedString>) -> bool {
        let QuestionnaireStage::Question(index) = self.stage else {
            return false;
        };
        self.answers[index] = Some(answer);
        if let Err(error) = validation {
            self.error = Some(error);
            return false;
        }
        self.error = None;
        self.stage = if index + 1 < self.answers.len() {
            QuestionnaireStage::Question(index + 1)
        } else {
            QuestionnaireStage::Review
        };
        true
    }

    pub fn can_go_back(&self) -> bool {
        match self.stage {
            QuestionnaireStage::Question(index) => index > 0,
            QuestionnaireStage::Review => !self.answers.is_empty(),
            QuestionnaireStage::Complete => true,
        }
    }

    /// Moves to the previous question, from Review to the last question, or
    /// from Complete to Review. It never clears answers.
    pub fn back(&mut self) -> bool {
        self.stage = match self.stage {
            QuestionnaireStage::Question(index) if index > 0 => {
                QuestionnaireStage::Question(index - 1)
            }
            QuestionnaireStage::Review if !self.answers.is_empty() => {
                QuestionnaireStage::Question(self.answers.len() - 1)
            }
            QuestionnaireStage::Complete => QuestionnaireStage::Review,
            _ => return false,
        };
        self.error = None;
        true
    }

    /// Commits the reviewed answers. Completion is local state, not a network
    /// submission; applications perform their own side effects after success.
    pub fn complete(&mut self) -> bool {
        if self.stage != QuestionnaireStage::Review {
            return false;
        }
        self.stage = QuestionnaireStage::Complete;
        true
    }
}

/// An unstyled semantic group for the active question, review, or result.
///
/// Supply a name describing the current stage, an existing Input or other
/// control, and app-owned navigation Buttons. Move focus to the next editor
/// after changing questions, or to a retained review/result control when that
/// editor unmounts. This group adds no automatic focus stop or keyboard binding.
#[derive(IntoElement)]
pub struct Questionnaire {
    base: ObservedElement<Stateful<Div>>,
}

impl Questionnaire {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            base: div()
                .id(id)
                .test_support()
                .role(Role::Group)
                .aria_label(label),
        }
    }
}

impl Styled for Questionnaire {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Questionnaire {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

impl RenderOnce for Questionnaire {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_answers_survive_back_review_and_completion() {
        let mut flow = QuestionnaireState::new(2);
        assert!(!flow.complete());
        assert!(!flow.back());
        assert!(!flow.submit_answer(String::new(), Err("A name is required".into())));
        assert_eq!(flow.stage(), QuestionnaireStage::Question(0));
        assert_eq!(flow.answer(0).map(String::as_str), Some(""));
        assert_eq!(
            flow.error().map(|error| error.as_ref()),
            Some("A name is required")
        );
        assert!(flow.submit_answer("First answer".to_string(), Ok(())));
        assert!(flow.back());
        assert_eq!(flow.answer(0).map(String::as_str), Some("First answer"));
        assert!(flow.submit_answer("Edited answer".to_string(), Ok(())));
        assert!(flow.submit_answer("Second answer".to_string(), Ok(())));
        assert_eq!(flow.stage(), QuestionnaireStage::Review);
        assert!(flow.back());
        assert_eq!(flow.answer(1).map(String::as_str), Some("Second answer"));
        assert!(flow.submit_answer("Final answer".to_string(), Ok(())));
        assert!(flow.complete());
        assert_eq!(flow.stage(), QuestionnaireStage::Complete);
        assert!(!flow.submit_answer("Ignored".to_string(), Ok(())));
        assert!(flow.back());
        assert_eq!(flow.stage(), QuestionnaireStage::Review);
        assert_eq!(flow.answer(0).map(String::as_str), Some("Edited answer"));
        assert_eq!(flow.answer(1).map(String::as_str), Some("Final answer"));
    }
}
