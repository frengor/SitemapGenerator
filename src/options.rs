use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use url::Url;

#[derive(Debug, Clone)]
pub struct Options {
    max_task_count: usize,
    remove_query_and_fragment: bool,
    max_recursion: usize,
    verbose: bool,
    verbose_sender: Option<UnboundedSender<Arc<Url>>>,
}

impl Options {
    #[inline]
    pub fn new(max_task_count: usize, remove_query_and_fragment: bool, max_recursion: usize, verbose: bool) -> Options {
        Options {
            max_task_count,
            remove_query_and_fragment,
            max_recursion,
            verbose,
            verbose_sender: None,
        }
    }

    #[inline]
    pub fn builder() -> OptionsBuilder {
        OptionsBuilder::new()
    }

    #[inline]
    pub fn max_task_count(&self) -> usize {
        self.max_task_count
    }

    #[inline]
    pub fn remove_query_and_fragment(&self) -> bool {
        self.remove_query_and_fragment
    }

    #[inline]
    pub fn max_recursion(&self) -> usize {
        self.max_recursion
    }

    #[inline]
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    #[inline]
    pub fn verbose_sender(&self) -> &Option<UnboundedSender<Arc<Url>>> {
        &self.verbose_sender
    }

    #[inline]
    pub fn set_verbose_sender(&mut self, verbose_sender: Option<UnboundedSender<Arc<Url>>>) {
        self.verbose_sender = verbose_sender;
    }
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        OptionsBuilder::default().build()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OptionsBuilder {
    max_task_count: usize,
    remove_query_and_fragment: bool,
    max_recursion: usize,
    verbose: bool,
}

impl OptionsBuilder {
    #[inline]
    pub fn new() -> OptionsBuilder {
        Default::default()
    }

    #[inline]
    pub fn build(self) -> Options {
        Options {
            max_task_count: self.max_task_count,
            remove_query_and_fragment: self.remove_query_and_fragment,
            max_recursion: self.max_recursion,
            verbose: self.verbose,
            verbose_sender: None,
        }
    }

    #[inline]
    pub fn set_max_task_count(mut self, max_task_count: usize) -> OptionsBuilder {
        self.max_task_count = max_task_count;
        self
    }

    #[inline]
    pub fn set_remove_query_and_fragment(mut self, remove_query_and_fragment: bool) -> OptionsBuilder {
        self.remove_query_and_fragment = remove_query_and_fragment;
        self
    }

    #[inline]
    pub fn set_max_recursion(mut self, max_recursion: usize) -> OptionsBuilder {
        self.max_recursion = max_recursion;
        self
    }

    #[inline]
    pub fn set_verbose(mut self, verbose: bool) -> OptionsBuilder {
        self.verbose = verbose;
        self
    }

    #[inline]
    pub fn max_task_count(&self) -> usize {
        self.max_task_count
    }

    #[inline]
    pub fn remove_query_and_fragment(&self) -> bool {
        self.remove_query_and_fragment
    }

    #[inline]
    pub fn max_recursion(&self) -> usize {
        self.max_recursion
    }

    #[inline]
    pub fn verbose(&self) -> bool {
        self.verbose
    }
}

impl Default for OptionsBuilder {
    #[inline]
    fn default() -> Self {
        OptionsBuilder {
            max_task_count: num_cpus::get(),
            remove_query_and_fragment: false,
            max_recursion: 50,
            verbose: false,
        }
    }
}
