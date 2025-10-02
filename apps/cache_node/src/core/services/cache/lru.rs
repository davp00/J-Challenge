use std::collections::HashMap;
use std::hash::Hash;

pub struct LruState<K> {
    capacity: usize,
    head: Option<K>,                           // MRU
    tail: Option<K>,                           // LRU
    links: HashMap<K, (Option<K>, Option<K>)>, // key -> (prev, next)
}

//NOTA: Como es un algoritmo que aún necesito interiorizar, por eso tantos comentarios

impl<K: Eq + Hash + Clone> LruState<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            head: None,
            tail: None,
            links: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.links.contains_key(key)
    }

    /// Quita un nodo de su posición actual (si existe) y devuelve (prev, next) antiguos
    pub fn detach(&mut self, key: &K) {
        if let Some((prev, next)) = self.links.get(key).cloned() {
            // Actualiza el anterior
            if let Some(ref p) = prev {
                if let Some(e) = self.links.get_mut(p) {
                    e.1 = next.clone();
                }
            } else {
                // era head
                self.head = next.clone();
            }

            // Actualiza el siguiente
            if let Some(ref n) = next {
                if let Some(e) = self.links.get_mut(n) {
                    e.0 = prev.clone();
                }
            } else {
                // era tail
                self.tail = prev.clone();
            }
        }
    }

    /// Inserta como head (MRU). Si ya existía, lo mueve a head.
    pub fn push_front(&mut self, key: K) {
        let existed = self.links.contains_key(&key);
        if existed {
            self.detach(&key);
        }

        let old_head = self.head.take();
        self.head = Some(key.clone());

        // Nuevo head apunta a prev=None, next=old_head
        self.links.insert(key.clone(), (None, old_head.clone()));

        // Arregla el prev del viejo head (si existía)
        if let Some(h) = old_head {
            if let Some(e) = self.links.get_mut(&h) {
                e.0 = Some(key.clone());
            }
        }

        // Si no había tail, también es tail
        if self.tail.is_none() {
            self.tail = Some(key);
        }
    }

    /// Saca el tail (LRU) y devuelve su clave
    pub fn pop_back(&mut self) -> Option<K> {
        let lru = self.tail.take()?;
        // El nuevo tail será el prev del antiguo tail
        let prev = self.links.get(&lru).and_then(|(p, _)| p.clone());
        if let Some(ref p) = prev {
            if let Some(e) = self.links.get_mut(p) {
                e.1 = None; // su next ahora es None
            }
        } else {
            // si no hay prev, también nos quedamos sin head
            self.head = None;
        }
        self.tail = prev;
        self.links.remove(&lru);
        Some(lru)
    }

    /// Marca como usado recientemente (mueve a head)
    pub fn touch(&mut self, key: K) {
        self.push_front(key);
    }

    pub fn remove(&mut self, key: &K) -> bool {
        if !self.links.contains_key(key) {
            return false;
        }
        self.detach(key);
        self.links.remove(key);
        true
    }

    pub fn over_capacity(&self) -> bool {
        self.links.len() > self.capacity
    }
}
