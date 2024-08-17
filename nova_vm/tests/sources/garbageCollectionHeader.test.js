const set = new Set();

const mapperFunction = (item, index) => {
    if (index === 0) {
        return { type: "None", value: item.value + 7 };
    } else if (item.type === "None") {
        return { type: "Thinking", value: item.value * 3 };
    } else if (item.type === "Thinking") {
        return { type: "Example", value: item.value - 1 };
    } else {
        const result = item.value > 13 ? { type: "None", value: item.value / 2 } : { type: "Example", value: item.value + 10 };
        set.add(result);
        return result;
    }
};


const runFunction = (start) => {
    if (!start) {
        start = new Array(256);
    }
    
    for (var i = 0; i < start.length; i++) {
        start[i] = { type: "None", value: 0 };
        if (i % 15 === 0) {
            set.add(start[i]);
        }
    }

    return start.map(mapperFunction).map(mapperFunction).map(mapperFunction).map(mapperFunction);
};
