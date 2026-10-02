// How many avatar colours the tokens define (`--avatar-1` ...).
export const avatarTones = 6

// The first letter or digit of a name, so '"Ann"' and ' ann' both give 'A'.
export const avatarInitial = (name: string): string => {
    const first = Array.from(name).find((character) => /[\p{L}\p{N}]/u.test(character))
    if (first === undefined) {
        return '?'
    }

    return first.toUpperCase()
}

// The same address always gets the same colour, so a sender is recognisable
// in the list before any text is read.
export const avatarTone = (seed: string): number => {
    let hash = 0
    for (const character of seed.trim().toLowerCase()) {
        hash = (hash * 31 + (character.codePointAt(0) ?? 0)) % 1_000_003
    }

    return hash % avatarTones + 1
}
