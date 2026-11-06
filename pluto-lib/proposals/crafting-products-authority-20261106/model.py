"""Symbolic protocol oracle ONLY; not Horizon, a transaction, or a runtime API.

Unique owner keys, comprehensive generation updates, exclusive commit, allocation
success, native effects, callback isolation and stack identity are assumptions.
They must be implemented and tested separately by an official host extension.
"""
from dataclasses import dataclass, field
from typing import Callable


@dataclass(frozen=True)
class Pair:
    output: str
    remainders: tuple[str | None, ...]


@dataclass
class Owner:
    # A symbolic host-issued unique lifetime key, NOT a pointer or recipe ID.
    key: str
    grid: tuple[str | None, ...]
    crafts_available: int = 3  # Abstract units, not native item counts.
    revision: int = 0
    destination_revision: int = 0
    recipe_revision: int = 0
    alive: bool = True
    committed: list[Pair] = field(default_factory=list)

    def snapshot(self):
        return (self.key, self.grid, self.crafts_available, self.revision,
                self.destination_revision, self.recipe_revision, self.alive)

    def edit_grid(self, grid):
        self.grid = grid
        self.revision += 1


class Operation:
    """Owns one symbolic pair; no mod-facing reusable commit ticket is exposed."""
    def __init__(self, owner: Owner, *, authoritative=True):
        self.expected = owner.snapshot()
        self.authoritative = authoritative
        self.state = 'new'
        self.pair = None

    def abort(self):
        self.state = 'aborted'
        self.pair = None
        return False

    def prepare(self, provider: Callable[[], Pair], *, admitted=True,
                supported=True, claimants=1):
        if self.state != 'new':
            return self.abort()
        if (not self.authoritative or not admitted or not self.expected[6]
                or self.expected[2] <= 0):
            return self.abort()
        if claimants == 0:
            self.state = 'native'
            return False  # Only indicates fallback; no native behavior is modeled.
        if claimants != 1 or not supported:
            return self.abort()
        self.state = 'preparing'
        try:
            pair = provider()
        except Exception:
            return self.abort()
        if self.state != 'preparing':  # A reentrant attempt aborted this operation.
            return self.abort()
        grid = self.expected[1]
        if (not isinstance(pair, Pair) or not pair.output
                or len(pair.remainders) != len(grid)
                or any(rest is not None and cell is None
                       for cell, rest in zip(grid, pair.remainders))):
            return self.abort()
        self.pair = pair
        self.state = 'ready'
        return True

    def commit(self, owner: Owner, *, allocation_succeeds=True,
               cancelled=False, effects_supported=True):
        if (self.state != 'ready' or owner.snapshot() != self.expected
                or not owner.alive or owner.crafts_available <= 0
                or not allocation_succeeds or cancelled or not effects_supported):
            return self.abort()
        # This one abstract transition ASSUMES an indivisible, infallible native
        # installation. Proving a real Menu/Crafter boundary is outside the model.
        pair, self.pair = self.pair, None
        owner.crafts_available -= 1
        owner.revision += 1
        owner.committed.append(pair)
        self.state = 'committed'
        return True


def preview(provider: Callable[[], Pair]):
    """Returns observation only. No Owner, Operation or authority is created."""
    return provider().output
