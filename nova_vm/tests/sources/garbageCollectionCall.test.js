try {
    runFunction(new Array(256));
} catch (err) {
    print("Error call");
    print(err.message);
    print("Error taking");
}