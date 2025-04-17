enum CompassDirection {
    North,
    East,
    South,
    West
}

globalThis.__enumCheck = {
    northValue: CompassDirection.North === 0,
    eastValue: CompassDirection.East === 1,
    southValue: CompassDirection.South === 2,
    westValue: CompassDirection.West === 3,

    zeroName: CompassDirection[0] === "North",
    oneName: CompassDirection[1] === "East",
    twoName: CompassDirection[2] === "South",
    threeName: CompassDirection[3] === "West",

    keys: Object.keys(CompassDirection)
};
