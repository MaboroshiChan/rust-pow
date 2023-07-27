use crate::queue::{Task, WorkQueue};
use digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::sync;
type Hash = GenericArray<u8, U32>;

#[derive(Debug, Clone)]
pub struct Block {
    prev_hash: Hash,
    pub(crate) generation: u64,
    pub(crate) difficulty: u8,
    pub(crate) data: String,
    pub(crate) proof: Option<u64>, // hash(proof) ends with .difficult zeros.
}

impl Block {
    pub fn initial(difficulty: u8) -> Block {
       //todo!(); // create and return a new initial block
        return Block {
            prev_hash: Hash::from([0u8; 32]),
            generation: 0, 
            difficulty,
            data: String::default(),
            proof: None //todo
       }
    }
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        write!(&mut output, "Block[{}]: {} (prev: {:02x}...)", self.generation, self.data, self.prev_hash).unwrap();
        output
    }
    /**
     * @return new block with higher generation, the same difficulty, data given in the argument, previous hash == b's hash
     */
    pub fn next(previous: &Block, data: String) -> Block {
        return Block {
            prev_hash: previous.hash(),
            generation: previous.generation + 1,
            difficulty: previous.difficulty,
            data,
            proof: None
        }
    }

    pub fn hash_string_for_proof(&self, proof: u64) -> String {
        let hash_code = self.hash_for_proof(proof);
        let mut output = String::new();
        write!(&mut output, "{:02x}:{}:{}:{}:{}", hash_code, self.generation, self.difficulty, self.data, self.proof.unwrap()).unwrap();
        output
    }

    pub fn hash_string(&self) -> String {
        // self.proof.unwrap() panics if block not mined
        let p = self.proof.unwrap();
        self.hash_string_for_proof(p)
    }

    pub fn hash_for_proof(&self, proof: u64) -> Hash {
        let new_hash = Sha256::new();
        //println!("data = {}, proof = {:?}", self.data, proof);
        let string_to_be_hashed: String = format!("{:02x}:{}:{}:{}:{}", self.prev_hash, self.generation, self.difficulty, self.data, proof);
        new_hash.chain_update(string_to_be_hashed.as_bytes()).finalize()
    }

    pub fn hash(&self) -> Hash {
        // self.proof.unwrap() panics if block not mined
        let p = self.proof.unwrap();
        self.hash_for_proof(p)
    }

    pub fn set_proof(self: &mut Block, proof: u64) {
        self.proof = Some(proof);
    }

    pub fn is_valid_for_proof(&self, proof: u64) -> bool {
        // would this block be valid if we set the proof to `proof`?
        let hash_code = self.hash_for_proof(proof);
        // we take last n bits of hash_code and check if they are all 0
        let n_bytes = self.difficulty as usize / 8;
        let remain_bits = self.difficulty as usize % 8;


        let last_n_bytes = &hash_code[hash_code.len() - n_bytes..];
        if n_bytes > 0 {
           // println!("last_n_bits = {:?}", last_n_bits);
            for each in last_n_bytes {
                if *each != 0u8 {
                    return false;
                }
            }
        }
        //check that the next byte is divisible by 1 << n_bits, using bit operations
        if remain_bits > 0 {
           // println!("n_bits = {}", n_bits);
            let last_byte = &hash_code[hash_code.len() - n_bytes - 1];
            let mask = (1 << remain_bits) - 1;
            if last_byte & mask != 0 {
                return false;
            }
        }
        return true;
    }

    pub fn is_valid(&self) -> bool {
        match self.proof {
            Some(proof) => self.is_valid_for_proof(proof),
            None => false
        }
    }

    // Mine in a very simple way: check sequentially until a valid hash is found.
    // This doesn't *need* to be used in any way, but could be used to do some mining
    // before your .mine is complete. Results should be the same as .mine (but slower).
    pub fn mine_serial(self: &mut Block) {
        let mut p = 0u64;
        while !self.is_valid_for_proof(p) {
            p += 1;
        }
        self.set_proof(p);
    }

    pub fn mine_range(self: &Block, workers: usize, start: u64, end: u64, chunks: u64) -> u64 {
        // With `workers` threads, check proof values in the given range, breaking up
	// into `chunks` tasks in a work queue. Return the first valid proof found.
        // HINTS:
        // - Create and use a queue::WorkQueue.
        // - Use sync::Arc to wrap a clone of self for sharing.
        let mut work_queue = WorkQueue::new(workers);
        let block = sync::Arc::new(self.clone());

        for i in 0..chunks {
            let task = MiningTask::new(block.clone()
                                       , start + i * (end - start) / chunks
                                       , start + (i + 1) * (end - start) / chunks);

            work_queue.enqueue(task).unwrap();
        }

        let ret = work_queue.recv();
        work_queue.shutdown();
        ret
    }

    pub fn mine_for_proof(self: &Block, workers: usize) -> u64 {
        let range_start: u64 = 0;
        let range_end: u64 = 8 * (1 << self.difficulty); // 8 * 2^(bits that must be zero)
        let chunks: u64 = 2345;
        self.mine_range(workers, range_start, range_end, chunks)
    }

    pub fn mine(self: &mut Block, workers: usize) {
        self.proof = Some(self.mine_for_proof(workers));
    }
}

struct MiningTask {
    block: sync::Arc<Block>,
    start: u64,
    end: u64
}

impl MiningTask {
    //  todo!(); // implement MiningTask::new(???) -> MiningTask
    fn new(block: sync::Arc<Block>, start: u64, end: u64) -> MiningTask {
        MiningTask {
            block,
            start,
            end
        }
    }
}

impl Task for MiningTask {
    type Output = u64;

    /**
     *tasks have an Output type that they return on success;
     *running a task produces an Option<Output>:
     *None if no result was found or Some(x) if it found a result x;
     */
    fn run(&self) -> Option<u64> {
        for i in self.start..self.end {
            if self.block.is_valid_for_proof(i) {
                return Some(i);
            }
        }
        None
    }
}
