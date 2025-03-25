class ClassWithPrivateStaticMethod {
    static #privateStaticMethod() {
        return 42;
    }

    static publicStaticMethod() {
        return ClassWithPrivateStaticMethod.#privateStaticMethod();
    }
}