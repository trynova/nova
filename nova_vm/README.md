## Project structure

The Nova VM source structure is as follows:

1. The `ecmascript` folder contains all code relating to the ECMAScript
   specification.
1. The `engine` folder contains Nova engine specific code such as bytecode.
1. The `heap` folder contains the setup for the heap of the VM and the direct
   APIs to work with it.

### ECMAScript folder structure

The ECMAScript folder will have its own READMEs to better describe various
details of the structure but the basic idea is to stay fairly close to the
ECMAScript specification text and its structure.

As an example, the `ecmascript/types` folder corresponds to the specification's
section 6, `ECMAScript Data Types and Values`. That section has two subsections,
6.1 `ECMAScript Language Types` and 6.2 `ECMAScript Specification Types`, which
then correspond to the `ecmascript/types/language` and `ecmascript/types/spec`
folders respectively.
