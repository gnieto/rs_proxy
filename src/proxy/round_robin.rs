pub struct RoundRobin<T: Copy> {
    list: Vec<T>,
    index: usize,
}

impl<T: Copy> RoundRobin<T> {
    pub fn new(list: Vec<T>) -> Self {
        RoundRobin {
            list: list,
            index: 0,
        }
    }

    pub fn get(&mut self) -> T {
        let r = self.list[self.index];
        self.index = (self.index + 1) % self.list.len();

        r
    }

    pub fn add(&mut self, element: T) {
        self.list.push(element);
    }

    pub fn remove(&mut self) {

    }
}
