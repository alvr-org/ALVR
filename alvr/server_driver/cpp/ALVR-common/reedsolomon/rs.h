#ifndef __RS_H_
#define __RS_H_

#ifdef __cplusplus
extern "C" {
#endif

	/* use small value to save memory */
#define DATA_SHARDS_MAX 255

	typedef struct _reed_solomon {
		int data_shards;
		int parity_shards;
		int shards;
		unsigned char* m;
		unsigned char* parity;
	} reed_solomon;

	/**
	 * MUST initial one time
	 * */
	void reed_solomon_init(void);

	reed_solomon* reed_solomon_new(int data_shards, int parity_shards);
	void reed_solomon_release(reed_solomon* rs);

	/**
	 * encode a big size of buffer
	 * input:
	 * rs
	 * nr_shards: assert(0 == nr_shards % rs->data_shards)
	 * shards[nr_shards][block_size]
	 * */
	int reed_solomon_encode(reed_solomon* rs, unsigned char** shards, int nr_shards, int block_size);

	/**
	 * reconstruct a big size of buffer
	 * input:
	 * rs
	 * nr_shards: assert(0 == nr_shards % rs->data_shards)
	 * shards[nr_shards][block_size]
	 * marks[nr_shards] marks as errors
	 * */
	int reed_solomon_reconstruct(reed_solomon* rs, unsigned char** shards, unsigned char* marks, int nr_shards, int block_size);

#ifdef __cplusplus
};
#endif
#endif

