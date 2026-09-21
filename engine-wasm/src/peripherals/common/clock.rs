use crate::peripherals::types::*;

// Pool and event storage limits (match JS pool cap of 10, with headroom for events)
const CLOCK_EVENT_POOL_SIZE: usize = 10;
const MAX_CLOCK_EVENTS: usize = 128;

#[derive(Clone, Copy)]
struct EventNode {
    cycles: u64,
    callback: EventTag,
    next: i32,
}

/// JS: ClockEvent — stores a reference to its SimulationClock plus a callback tag.
/// schedule/unschedule delegate to the owning clock's linked-list methods.
pub struct ClockEvent {
    pub clock: *mut SimulationClock,
    pub callback: EventTag,
}

impl ClockEvent {
    /// JS: constructor(cpuVal, tmpVal) — cpuVal = this (SimulationClock), tmpVal = callback
    pub fn new(clock: *mut SimulationClock, callback: EventTag) -> Self {
        ClockEvent { clock, callback }
    }

    /// JS: schedule(cpuVal) — unschedules first, then calls clock.schedule(callback, delta)
    /// cpuVal maps to `delta` (nanoseconds)
    pub fn schedule(&mut self, ctx: &mut CpuContext, delta: u64) -> EventTag {
        self.unschedule(ctx);
        let clock = unsafe { &mut *self.clock };
        clock.schedule(ctx, self.callback, delta);
        self.callback
    }

    /// JS: unschedule() — calls clock.unschedule(callback)
    pub fn unschedule(&mut self, _ctx: &mut CpuContext) {
        let clock = unsafe { &mut *self.clock };
        clock.unschedule(self.callback);
    }
}

/// JS: SimulationClock — manages a sorted linked-list of events with a recycle pool.
/// tick() fires exactly one pending event (the head).  skipToNextEvent() either fires
/// the head event (jumping CPU time to it) or advances CPU time to max_nanos.
pub struct SimulationClock {
    pub frequency: u32,
    pub next_clock_event: i32,
    pub clock_event_pool: [i32; CLOCK_EVENT_POOL_SIZE],
    pub pool_count: i32,
    nodes: [EventNode; MAX_CLOCK_EVENTS],
    node_count: i32,
}

impl SimulationClock {
    /// JS: constructor(cpuVal = 125e6, tmpVal = { cycles: 0 })
    ///   cpuVal = frequency, tmpVal = cpu reference (cpu.cycles -> ctx.cpu_tick)
    pub fn new(frequency: u32) -> Self {
        SimulationClock {
            frequency,
            next_clock_event: -1,
            clock_event_pool: [-1; CLOCK_EVENT_POOL_SIZE],
            pool_count: 0,
            nodes: [EventNode { cycles: 0, callback: EventTag::FrcTimerAlarm { channel: 0 }, next: -1 }; MAX_CLOCK_EVENTS],
            node_count: 0,
        }
    }

    /// JS: get micros()  —  this.nanos / 1e3
    pub fn micros(&self, ctx: &CpuContext) -> u64 {
        self.nanos(ctx) / 1_000
    }

    /// JS: get millis()  —  (this.cpu.cycles / this.frequency) * 1e3
    pub fn millis(&self, ctx: &CpuContext) -> u64 {
        (ctx.cpu_tick as u128 * 1_000 / self.frequency as u128) as u64
    }

    /// JS: get nanos()  —  (this.cpu.cycles / this.frequency) * 1e9
    pub fn nanos(&self, ctx: &CpuContext) -> u64 {
        (ctx.cpu_tick as u128 * 1_000_000_000 / self.frequency as u128) as u64
    }

    /// JS: get ticks()  —  return this.nanos
    pub fn ticks(&self, ctx: &CpuContext) -> u64 {
        self.nanos(ctx)
    }

    /// JS: createEvent(cpuVal)  —  cpuVal = callback tag
    /// Returns a ClockEvent whose clock pointer points to self.
    pub fn create_event(&mut self, callback: EventTag) -> ClockEvent {
        ClockEvent::new(self as *mut SimulationClock, callback)
    }

    /// JS: schedule(cpuVal, tmpVal)
    ///   cpuVal = callback, tmpVal = delta in nanoseconds
    ///   idxVal = Math.round((tmpVal / 1e9) * this.frequency)  [cycles]
    ///   return this.addEventCycles(cpuVal, idxVal)
    pub fn schedule(&mut self, ctx: &CpuContext, callback: EventTag, nanos: u64) -> EventTag {
        // Math.round((nanos / 1e9) * freq) via integer arithmetic
        let idx_val = ((nanos as u128 * self.frequency as u128) + 500_000_000) / 1_000_000_000;
        self.add_event_cycles(ctx, callback, idx_val as u64)
    }

    /// JS: unschedule(cpuVal)  —  cpuVal = callback
    /// Walks the linked list, splices out the matching node and recycles it.
    pub fn unschedule(&mut self, callback: EventTag) -> bool {
        let mut prev: i32 = -1;
        let mut curr = self.next_clock_event;
        while curr != -1 {
            if self.nodes[curr as usize].callback == callback {
                if prev == -1 {
                    self.next_clock_event = self.nodes[curr as usize].next;
                } else {
                    self.nodes[prev as usize].next = self.nodes[curr as usize].next;
                }
                if (self.pool_count as usize) < CLOCK_EVENT_POOL_SIZE {
                    self.clock_event_pool[self.pool_count as usize] = curr;
                    self.pool_count += 1;
                }
                return true;
            }
            prev = curr;
            curr = self.nodes[curr as usize].next;
        }
        false
    }

    /// JS: addEventCycles(cpuVal, tmpVal)
    ///   tmpVal = delta_cycles (after Math.round and Math.max(1, ...))
    ///   target = this.cpu.cycles + Math.max(1, tmpVal)
    ///   size = idxVal.pop() ?? { cycles: target, callback: cpuVal, next: null }
    ///   Inserts into sorted linked list, returns cpuVal
    pub fn add_event_cycles(&mut self, ctx: &CpuContext, callback: EventTag, delta_cycles: u64) -> EventTag {
        let target = ctx.cpu_tick + core::cmp::max(1u64, delta_cycles);

        // Get a node from the pool, or allocate a new slot
        let idx = if self.pool_count > 0 {
            self.pool_count -= 1;
            self.clock_event_pool[self.pool_count as usize]
        } else if (self.node_count as usize) < MAX_CLOCK_EVENTS {
            let i = self.node_count;
            self.node_count += 1;
            i
        } else {
            // No slots available — return without scheduling
            return callback;
        };

        self.nodes[idx as usize] = EventNode {
            cycles: target,
            callback,
            next: -1,
        };

        // Insert sorted by cycles (ascending)
        let mut prev: i32 = -1;
        let mut curr = self.next_clock_event;
        while curr != -1 && self.nodes[curr as usize].cycles < target {
            prev = curr;
            curr = self.nodes[curr as usize].next;
        }

        if prev == -1 {
            self.next_clock_event = idx;
        } else {
            self.nodes[prev as usize].next = idx;
        }
        self.nodes[idx as usize].next = curr;

        callback
    }

    /// JS: tick()
    ///   let { nextClockEvent: cpuVal } = this;
    ///   cpuVal && cpuVal.cycles <= this.cpu.cycles &&
    ///     ((this.nextClockEvent = cpuVal.next),
    ///      this.clockEventPool.length < 10 && this.clockEventPool.push(cpuVal),
    ///      cpuVal.callback());
    /// Fires EXACTLY ONE pending event if its cycle has been reached.
    /// The `handler` closure is called with the event's tag so the caller
    /// can dispatch it (matching JS callback()). Use `|_| {}` to discard.
    pub fn tick<F: FnMut(EventTag)>(&mut self, ctx: &CpuContext, mut handler: F) {
        let head = self.next_clock_event;
        if head != -1 && self.nodes[head as usize].cycles <= ctx.cpu_tick {
            self.next_clock_event = self.nodes[head as usize].next;
            if (self.pool_count as usize) < CLOCK_EVENT_POOL_SIZE {
                self.clock_event_pool[self.pool_count as usize] = head;
                self.pool_count += 1;
            }
            handler(self.nodes[head as usize].callback);
        }
    }

    /// JS: skipToNextEvent(cpuVal = 0)  —  cpuVal = max_nanos
    ///   let { nextClockEvent: tmpVal, frequency: idxVal } = this;
    ///   tmpVal && (tmpVal.cycles / idxVal) * 1e9 <= cpuVal
    ///     ? (jump CPU to event, fire)
    ///     : cpuVal > this.nanos && advance CPU to max_nanos
    /// The `handler` closure is called with the event's tag if one fires.
    pub fn skip_to_next_event<F: FnMut(EventTag)>(&mut self, ctx: &mut CpuContext, max_nanos: u64, mut handler: F) {
        let head = self.next_clock_event;
        if head != -1 {
            // (cycles / frequency) * 1e9  —  event time in nanoseconds
            let event_nanos =
                (self.nodes[head as usize].cycles as u128 * 1_000_000_000 / self.frequency as u128) as u64;
            if event_nanos <= max_nanos {
                ctx.cpu_tick = self.nodes[head as usize].cycles;
                self.next_clock_event = self.nodes[head as usize].next;
                if (self.pool_count as usize) < CLOCK_EVENT_POOL_SIZE {
                    self.clock_event_pool[self.pool_count as usize] = head;
                    self.pool_count += 1;
                }
                handler(self.nodes[head as usize].callback);
                return;
            }
        }
        // No event to fire, or event time > max_nanos
        // Advance CPU to max_nanos if it's ahead of current time
        if max_nanos > self.nanos(ctx) {
            // this.cpu.cycles = Math.round((this.frequency / 1e9) * cpuVal)
            // Math.round((freq / 1e9) * max_nanos) via integer arithmetic
            ctx.cpu_tick = (((max_nanos as u128 * self.frequency as u128) + 500_000_000) / 1_000_000_000) as u64;
        }
    }

    /// JS: get nextClockEventCycles()
    ///   return this.nextClockEvent?.cycles ?? 0;
    pub fn next_clock_event_cycles(&self) -> u64 {
        if self.next_clock_event == -1 {
            0
        } else {
            self.nodes[self.next_clock_event as usize].cycles
        }
    }
}
