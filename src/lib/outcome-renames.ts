export function outcomeRenames(originals: (string | null)[], outcomes: string[]) {
  return Object.fromEntries(originals.flatMap((original, i) =>
    original !== null && original !== outcomes[i] ? [[original, outcomes[i]]] : []
  ))
}
