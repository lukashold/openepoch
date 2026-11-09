// Detached replay of the original per-overlay allocator. Mesh counts are supplied
// by the caller. Allocation order is independent of the texture and owner.
const minimumCapacity = 2000;

class VertexBuffer {
  constructor(capacity) {
    this.capacity = capacity;
    this.free = [{ start: 0, length: capacity }];
    this.items = [];
  }

  allocate(length) {
    let best = -1;
    for (const [index, span] of this.free.entries()) {
      if (span.length >= length && (best < 0 || span.length < this.free[best].length)) best = index;
    }
    if (best < 0) return undefined;
    const span = this.free[best];
    const start = span.start;
    if (span.length === length) {
      this.free.splice(best, 1);
    } else {
      span.start += length;
      span.length -= length;
    }
    return start;
  }

  release(start, length) {
    const right = this.free.find(span => span.start === start + length);
    const left = this.free.find(span => span.start + span.length === start);
    if (left && right) {
      left.length += length + right.length;
      this.free.splice(this.free.indexOf(right), 1);
    } else if (left) {
      left.length += length;
    } else if (right) {
      right.start = start;
      right.length += length;
    } else {
      this.free.unshift({ start, length });
    }
  }
}

export class BorderBufferCache {
  #buffers = [];
  #pending = [];
  #items = new Map();

  register(id, vertices, indices) {
    if (this.#items.has(id) || !Number.isInteger(vertices) || vertices < 1 || vertices > 0xffff ||
        !Number.isInteger(indices) || indices < 3 || indices % 3 !== 0) {
      throw new RangeError("Invalid detached decal mesh");
    }
    const item = { id, vertices, indices };
    this.#items.set(id, item);
    this.#pending.push(item);
  }

  remove(id) {
    const item = this.#items.get(id);
    if (!item) return;
    if (item.buffer === undefined) {
      this.#pending.splice(this.#pending.indexOf(item), 1);
    } else {
      const buffer = this.#buffers[item.buffer];
      buffer.items.splice(buffer.items.indexOf(item), 1);
      buffer.release(item.offset, item.vertices);
    }
    this.#items.delete(id);
  }

  compile(rebuild = false) {
    if (rebuild) {
      for (const buffer of this.#buffers) {
        for (const item of buffer.items) {
          buffer.release(item.offset, item.vertices);
          this.#pending.push(item);
        }
        buffer.items.length = 0;
      }
    }

    const newVertexBytes = [];
    const queries = this.#pending.length;
    while (this.#pending.length) {
      const item = this.#pending.pop();
      let bucket = 0;
      for (; bucket < this.#buffers.length; ++bucket) {
        item.offset = this.#buffers[bucket].allocate(item.vertices);
        if (item.offset !== undefined) break;
      }
      if (bucket === this.#buffers.length) {
        const buffer = new VertexBuffer(Math.max(minimumCapacity, item.vertices));
        this.#buffers.push(buffer);
        item.offset = buffer.allocate(item.vertices);
        newVertexBytes.push(buffer.capacity * 24);
      }
      item.buffer = bucket;
      this.#buffers[bucket].items.unshift(item);
    }

    let indexCount = 0;
    const draws = this.#buffers.flatMap(buffer => buffer.items.map(item => {
      const record = { ...item, firstIndex: indexCount };
      indexCount += item.indices;
      return record;
    }));
    return { draws, queries, buffers: this.#buffers.length, newVertexBytes, indexBytes: 2 * indexCount };
  }
}
