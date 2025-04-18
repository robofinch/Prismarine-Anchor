// Reading world files
pub mod zip_env;
pub mod bedrock;
// mod java;

pub mod translation;


// Right now this is focused on Bedrock Edition, and only the newest version.
// Support *will* be added for the newest version of Java, and then I'll work
// back from there at some point after I get visible output (a MVP of sorts).


// Also, I'll probably use PosixMemEnv and MemEnv for both bedrock and java;
// even though Java has nothing to do with rusty_leveldb, its env trait is convenient.
// Probably.

// Specifically, in the case of a native app, it can use a PosixMemEnv, or MemEnv with root "".
// in the case of wasm, it can be MemEnv.
// Definitely want to support converting MemEnv to and from zip
// Might also want to support the ability to read in a world folder with the webkitdirectory
// option of the web filesystem API, eventually.
