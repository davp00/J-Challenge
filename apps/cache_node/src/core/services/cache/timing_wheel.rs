use std::{
    hash::Hash,
    sync::atomic::{AtomicU64, Ordering},
};

use dashmap::{DashMap, DashSet};

use crate::core::services::cache::Cache;

pub struct TimingWheel<K>
where
    K: Hash + Send + Sync + 'static,
{
    /// Slots circulares: cada uno contiene claves programadas para ese tick.
    slots: Vec<DashSet<K>>,
    /// Índice inverso: clave -> índice del slot donde está actualmente.
    index: DashMap<K, usize>,
    /// Milisegundos por tick.
    pub tick_ms: u64,
    /// Cantidad de slots.
    size: usize,
    /// Número absoluto de tick (crece sin tope; usamos % size para el slot).
    pub cursor: AtomicU64,
}

//Nota Hay muchos comentarios porque igual es un algoritmo que no domino del todo
impl<K> TimingWheel<K>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
{
    pub fn new(size: usize, tick_ms: u64, start_ms: u64) -> Self {
        assert!(
            size.is_power_of_two(),
            "size debe ser potencia de 2 para mod rápido (opcional)"
        );

        let mut slots = Vec::with_capacity(size);

        for _ in 0..size {
            slots.push(DashSet::new());
        }

        let start_tick = start_ms / tick_ms;

        Self {
            slots,
            index: DashMap::new(),
            tick_ms,
            size,
            cursor: AtomicU64::new(start_tick),
        }
    }

    /// Calcula el slot para un `expires_at` absoluto en ms.
    #[inline]
    pub fn slot_for(&self, expires_at_ms: u64) -> (u64, usize) {
        let t = expires_at_ms / self.tick_ms;
        let slot = (t as usize) & (self.size - 1); // size potencia de 2 -> mod rápido
        (t, slot)
    }

    /// Agenda (o re-agenda) una clave para su expiración.
    pub fn schedule(&self, key: K, expires_at_ms: u64) {
        // Determina el slot destino
        let (_t, slot_idx) = self.slot_for(expires_at_ms);

        // Si ya existía, quitar del slot anterior
        if let Some(prev) = self.index.get(&key) {
            let prev_idx = *prev;
            if prev_idx != slot_idx {
                if let Some(set) = self.slots.get(prev_idx) {
                    set.remove(&key);
                }
                drop(prev);
                self.index.insert(key.clone(), slot_idx);
                self.slots[slot_idx].insert(key);
                return;
            }
            // Ya está en el slot correcto
            return;
        }

        // Nuevo registro
        self.index.insert(key.clone(), slot_idx);
        self.slots[slot_idx].insert(key);
    }

    /// Desagenda una clave si existe.
    pub fn deschedule(&self, key: &K) {
        if let Some((k, slot_idx)) = self.index.remove(key)
            && let Some(set) = self.slots.get(slot_idx)
        {
            set.remove(&k);
        }
    }

    /// Avanza el cursor hasta `target_ms`, drenando los slots intermedios.
    /// Llama a `invalidate_if_expired` para cada clave en el slot.
    pub fn advance_to<V: Send + Sync + 'static>(
        &self,
        target_ms: u64,
        cache: &Cache<K, V>,
        invalidate_if_expired: impl Fn(&Cache<K, V>, &K, u64),
    ) {
        let target_tick = target_ms / self.tick_ms;
        let mut cur = self.cursor.load(Ordering::Relaxed);

        while cur < target_tick {
            let slot_idx = (cur as usize) & (self.size - 1);

            // Drenar el slot actual
            if let Some(set) = self.slots.get(slot_idx) {
                // Para evitar bloquear el set mientras invalidamos,
                // copiamos las claves a un Vec y luego removemos del índice.
                let keys: Vec<K> = set.iter().map(|r| r.clone()).collect();

                for k in keys {
                    // Sacar del slot + índice inverso
                    set.remove(&k);
                    let _ = self.index.remove(&k);

                    // Validar expiración real y, si aplica, invalidar
                    invalidate_if_expired(cache, &k, target_ms);
                }
            }

            cur += 1;
            self.cursor.store(cur, Ordering::Relaxed);
        }
    }
}
