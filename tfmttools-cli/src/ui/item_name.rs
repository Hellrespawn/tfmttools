pub struct ItemName<'i> {
    single: &'i str,
    plural: Option<&'i str>,
}

impl<'i> ItemName<'i> {
    pub fn simple(single: &'i str) -> Self {
        Self { single, plural: None }
    }

    #[allow(dead_code)]
    pub fn new(single: &'i str, plural: &'i str) -> Self {
        Self { single, plural: Some(plural) }
    }

    pub fn single(&self) -> String {
        self.single.to_owned()
    }

    pub fn plural(&self) -> String {
        self.plural.map_or(
            format!("{}s", self.single()),
            std::borrow::ToOwned::to_owned,
        )
    }

    pub fn by_amount(&self, amount: usize) -> String {
        if amount == 1 { self.single() } else { self.plural() }
    }
}

impl Default for ItemName<'_> {
    fn default() -> Self {
        Self { single: "item", plural: None }
    }
}
