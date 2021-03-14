/*
 * fec.c -- forward error correction based on Vandermonde matrices
 *
 * (C) 1997-98 Luigi Rizzo (luigi@iet.unipi.it)
 * (C) 2001 Alain Knaff (alain@knaff.lu)
 * (C) 2017 Iwan Timmer (irtimmer@gmail.com)
 *
 * Portions derived from code by Phil Karn (karn@ka9q.ampr.org),
 * Robert Morelos-Zaragoza (robert@spectra.eng.hawaii.edu) and Hari
 * Thirumoorthy (harit@spectra.eng.hawaii.edu), Aug 1995
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above
 *    copyright notice, this list of conditions and the following
 *    disclaimer in the documentation and/or other materials
 *    provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY,
 * OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA,
 * OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR
 * TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <assert.h>
#include "rs.h"

#ifdef _MSC_VER
#define alloca(x) _alloca(x)
#endif

typedef unsigned char gf;

#define GF_BITS  8
#define GF_PP "101110001"
#define GF_SIZE ((1 << GF_BITS) - 1)

#define SWAP(a,b,t) {t tmp; tmp=a; a=b; b=tmp;}

/*
 * USE_GF_MULC, GF_MULC0(c) and GF_ADDMULC(x) can be used when multiplying
 * many numbers by the same constant. In this case the first
 * call sets the constant, and others perform the multiplications.
 * A value related to the multiplication is held in a local variable
 * declared with USE_GF_MULC . See usage in addmul1().
 */
#define USE_GF_MULC register gf * __gf_mulc_
#define GF_MULC0(c) __gf_mulc_ = &gf_mul_table[(c)<<8]
#define GF_ADDMULC(dst, x) dst ^= __gf_mulc_[x]
#define GF_MULC(dst, x) dst = __gf_mulc_[x]

#define gf_mul(x,y) gf_mul_table[(x<<8)+y]

/*
 * To speed up computations, we have tables for logarithm, exponent
 * multiplication and inverse of a number.
 */
static gf gf_exp[2*GF_SIZE];
static int gf_log[GF_SIZE + 1];
static gf inverse[GF_SIZE+1];
#ifdef _MSC_VER
static gf __declspec(align (256)) gf_mul_table[(GF_SIZE + 1)*(GF_SIZE + 1)];
#else
static gf gf_mul_table[(GF_SIZE + 1)*(GF_SIZE + 1)] __attribute__((aligned (256)));
#endif

/*
 * modnn(x) computes x % GF_SIZE, where GF_SIZE is 2**GF_BITS - 1,
 * without a slow divide.
 */
static inline gf modnn(int x) {
    while (x >= GF_SIZE) {
        x -= GF_SIZE;
        x = (x >> GF_BITS) + (x & GF_SIZE);
    }
    return x;
}

static void addmul(gf *dst1, gf *src1, gf c, int sz) {
    USE_GF_MULC;
    if (c != 0) {
        register gf *dst = dst1, *src = src1;
        gf *lim = &dst[sz];

        GF_MULC0(c);
        for (; dst < lim; dst++, src++)
            GF_ADDMULC(*dst, *src);
    }
}

static void mul(gf *dst1, gf *src1, gf c, int sz) {
    USE_GF_MULC;
    if (c != 0) {
        register gf *dst = dst1, *src = src1;
        gf *lim = &dst[sz];
        GF_MULC0(c);
        for (; dst < lim; dst++, src++)
            GF_MULC(*dst , *src);
    } else
        memset(dst1, 0, c);
}

/* y = a.dot(b) */
static gf* multiply1(gf *a, int ar, int ac, gf *b, int br, int bc) {
    gf *new_m, tg;
    int r, c, i, ptr = 0;

    assert(ac == br);
    new_m = (gf*) calloc(1, ar*bc);
    if (NULL != new_m) {

        /* this multiply is slow */
        for (r = 0; r < ar; r++) {
            for (c = 0; c < bc; c++) {
                tg = 0;
                for (i = 0; i < ac; i++)
                    tg ^= gf_mul(a[r*ac+i], b[i*bc+c]);

                new_m[ptr++] = tg;
            }
        }
    }

    return new_m;
}

static void init_mul_table(void) {
    int i, j;
    for (i=0; i< GF_SIZE+1; i++)
    for (j=0; j< GF_SIZE+1; j++)
        gf_mul_table[(i<<8)+j] = gf_exp[modnn(gf_log[i] + gf_log[j]) ] ;

    for (j=0; j< GF_SIZE+1; j++)
        gf_mul_table[j] = gf_mul_table[j<<8] = 0;
}

/*
 * initialize the data structures used for computations in GF.
 */
static void generate_gf(void) {
    int i;
    gf mask;

    mask = 1;
    gf_exp[GF_BITS] = 0;
    /*
     * first, generate the (polynomial representation of) powers of \alpha,
     * which are stored in gf_exp[i] = \alpha ** i .
     * At the same time build gf_log[gf_exp[i]] = i .
     * The first GF_BITS powers are simply bits shifted to the left.
     */
    for (i = 0; i < GF_BITS; i++, mask <<= 1) {
        gf_exp[i] = mask;
        gf_log[gf_exp[i]] = i;
        /*
        * If GF_PP[i] == 1 then \alpha ** i occurs in poly-repr
        * gf_exp[GF_BITS] = \alpha ** GF_BITS
        */
        if (GF_PP[i] == '1')
            gf_exp[GF_BITS] ^= mask;
    }
    /*
     * now gf_exp[GF_BITS] = \alpha ** GF_BITS is complete, so can als
     * compute its inverse.
     */
    gf_log[gf_exp[GF_BITS]] = GF_BITS;
    /*
     * Poly-repr of \alpha ** (i+1) is given by poly-repr of
     * \alpha ** i shifted left one-bit and accounting for any
     * \alpha ** GF_BITS term that may occur when poly-repr of
     * \alpha ** i is shifted.
     */
    mask = 1 << (GF_BITS - 1) ;
    for (i = GF_BITS + 1; i < GF_SIZE; i++) {
        if (gf_exp[i - 1] >= mask)
            gf_exp[i] = gf_exp[GF_BITS] ^ ((gf_exp[i - 1] ^ mask) << 1);
        else
            gf_exp[i] = gf_exp[i - 1] << 1;

        gf_log[gf_exp[i]] = i;
    }
    /*
     * log(0) is not defined, so use a special value
     */
    gf_log[0] = GF_SIZE;
    /* set the extended gf_exp values for fast multiply */
    for (i = 0; i < GF_SIZE; i++)
        gf_exp[i + GF_SIZE] = gf_exp[i];

    /*
     * again special cases. 0 has no inverse. This used to
     * be initialized to GF_SIZE, but it should make no difference
     * since noone is supposed to read from here.
     */
    inverse[0] = 0;
    inverse[1] = 1;
    for (i=2; i<=GF_SIZE; i++)
        inverse[i] = gf_exp[GF_SIZE-gf_log[i]];
}

/*
 * invert_mat() takes a matrix and produces its inverse
 * k is the size of the matrix.
 * (Gauss-Jordan, adapted from Numerical Recipes in C)
 * Return non-zero if singular.
 */
static int invert_mat(gf *src, int k) {
    gf c, *p;
    int irow, icol, row, col, i, ix;

    int error = 1;
    int *indxc = (int*)alloca(k*sizeof(int));
    int *indxr = (int*)alloca(k*sizeof(int));
    int *ipiv = (int*)alloca(k*sizeof(int));
    gf *id_row = (gf*)alloca(k*sizeof(gf));

    memset(id_row, 0, k*sizeof(gf));
    /*
     * ipiv marks elements already used as pivots.
     */
    for (i = 0; i < k; i++)
        ipiv[i] = 0;

    for (col = 0; col < k; col++) {
        gf *pivot_row;
        /*
         * Zeroing column 'col', look for a non-zero element.
         * First try on the diagonal, if it fails, look elsewhere.
         */
        irow = icol = -1;
        if (ipiv[col] != 1 && src[col*k + col] != 0) {
            irow = col;
            icol = col;
            goto found_piv;
        }
        for (row = 0; row < k; row++) {
            if (ipiv[row] != 1) {
                for (ix = 0; ix < k; ix++) {
                    if (ipiv[ix] == 0) {
                        if (src[row*k + ix] != 0) {
                            irow = row;
                            icol = ix;
                            goto found_piv;
                        }
                    } else if (ipiv[ix] > 1) {
                        fprintf(stderr, "singular matrix\n");
                        goto fail;
                    }
                }
            }
        }
        if (icol == -1) {
            fprintf(stderr, "XXX pivot not found!\n");
            goto fail ;
        }
        
        found_piv:
        ++(ipiv[icol]);
        /*
         * swap rows irow and icol, so afterwards the diagonal
         * element will be correct. Rarely done, not worth
         * optimizing.
         */
        if (irow != icol) {
            for (ix = 0; ix < k; ix++) {
                SWAP(src[irow*k + ix], src[icol*k + ix], gf);
            }
        }
        indxr[col] = irow;
        indxc[col] = icol;
        pivot_row = &src[icol*k];
        c = pivot_row[icol];
        if (c == 0) {
            fprintf(stderr, "singular matrix 2\n");
            goto fail;
        } else if (c != 1 ) {
            /*
             * this is done often , but optimizing is not so
             * fruitful, at least in the obvious ways (unrolling)
             */
            c = inverse[ c ];
            pivot_row[icol] = 1;
            for (ix = 0; ix < k; ix++)
                pivot_row[ix] = gf_mul(c, pivot_row[ix]);
        }
        /*
         * from all rows, remove multiples of the selected row
         * to zero the relevant entry (in fact, the entry is not zero
         * because we know it must be zero).
         * (Here, if we know that the pivot_row is the identity,
         * we can optimize the addmul).
         */
        id_row[icol] = 1;
        if (memcmp(pivot_row, id_row, k*sizeof(gf)) != 0) {
            for (p = src, ix = 0 ; ix < k ; ix++, p += k) {
                if (ix != icol) {
                    c = p[icol];
                    p[icol] = 0;
                    addmul(p, pivot_row, c, k);
                }
            }
        }
        id_row[icol] = 0;
    }
    for (col = k-1 ; col >= 0 ; col-- ) {
        if (indxr[col] <0 || indxr[col] >= k)
            fprintf(stderr, "AARGH, indxr[col] %d\n", indxr[col]);
        else if (indxc[col] <0 || indxc[col] >= k)
            fprintf(stderr, "AARGH, indxc[col] %d\n", indxc[col]);
        else
            if (indxr[col] != indxc[col] ) {
                for (row = 0 ; row < k ; row++ )
                    SWAP( src[row*k + indxr[col]], src[row*k + indxc[col]], gf);
            }
    }
    error = 0;

    fail:
    return error ;
}

/*
 * Not check for input params
 * */
static gf* sub_matrix(gf* matrix, int rmin, int cmin, int rmax, int cmax,  int nrows, int ncols) {
    int i, j, ptr = 0;
    gf* new_m = (gf*) malloc((rmax-rmin) * (cmax-cmin));
    if (NULL != new_m) {
        for (i = rmin; i < rmax; i++) {
            for (j = cmin; j < cmax; j++) {
                new_m[ptr++] = matrix[i*ncols + j];
            }
        }
    }

    return new_m;
}

/* copy from golang rs version */
static inline int code_some_shards(gf* matrixRows, gf** inputs, gf** outputs, int dataShards, int outputCount, int byteCount) {
    gf* in;
    int iRow, c;
    for (c = 0; c < dataShards; c++) {
        in = inputs[c];
        for (iRow = 0; iRow < outputCount; iRow++) {
            if (0 == c)
                mul(outputs[iRow], in, matrixRows[iRow*dataShards+c], byteCount);
            else
                addmul(outputs[iRow], in, matrixRows[iRow*dataShards+c], byteCount);
        }
    }

    return 0;
}

void reed_solomon_init(void) {
    generate_gf();
    init_mul_table();
}

reed_solomon* reed_solomon_new(int data_shards, int parity_shards) {
    gf* vm = NULL;
    gf* top = NULL;
    int err = 0;
    reed_solomon* rs = NULL;

    do {
        rs = (reed_solomon *)malloc(sizeof(reed_solomon));
        if (NULL == rs)
            return NULL;

        rs->data_shards = data_shards;
        rs->parity_shards = parity_shards;
        rs->shards = (data_shards + parity_shards);
        rs->m = NULL;
        rs->parity = NULL;

        if (rs->shards > DATA_SHARDS_MAX || data_shards <= 0 || parity_shards <= 0) {
            err = 1;
            break;
        }

        vm = (gf*)malloc(data_shards * rs->shards);

        if (NULL == vm) {
            err = 2;
            break;
        }
        
        int ptr = 0;
        for (int row = 0; row < rs->shards; row++) {
            for (int col = 0; col < data_shards; col++)
                vm[ptr++] = row == col ? 1 : 0;
        }

        top = sub_matrix(vm, 0, 0, data_shards, data_shards, rs->shards, data_shards);
        if (NULL == top) {
            err = 3;
            break;
        }

        err = invert_mat(top, data_shards);
        assert(0 == err);

        rs->m = multiply1(vm, rs->shards, data_shards, top, data_shards, data_shards);
        if (NULL == rs->m) {
            err = 4;
            break;
        }

        for (int j = 0; j < parity_shards; j++) {
            for (int i = 0; i < data_shards; i++)
                rs->m[(data_shards + j)*data_shards + i] = inverse[(parity_shards + i) ^ j];
        }

        rs->parity = sub_matrix(rs->m, data_shards, 0, rs->shards, data_shards, rs->shards, data_shards);
        if (NULL == rs->parity) {
            err = 5;
            break;
        }

        free(vm);
        free(top);
        vm = NULL;
        top = NULL;
        return rs;

    } while(0);

    fprintf(stderr, "err=%d\n", err);
    if (NULL != vm)
        free(vm);

    if (NULL != top)
        free(top);

    if (NULL != rs) {
        if (NULL != rs->m)
            free(rs->m);

        if (NULL != rs->parity)
            free(rs->parity);

        free(rs);
    }

    return NULL;
}

void reed_solomon_release(reed_solomon* rs) {
    if (NULL != rs) {
        if (NULL != rs->m)
            free(rs->m);

        if (NULL != rs->parity)
            free(rs->parity);

        free(rs);
    }
}

/**
 * decode one shard
 * input:
 * rs
 * original data_blocks[rs->data_shards][block_size]
 * dec_fec_blocks[nr_fec_blocks][block_size]
 * fec_block_nos: fec pos number in original fec_blocks
 * erased_blocks: erased blocks in original data_blocks
 * nr_fec_blocks: the number of erased blocks
 * */
static int reed_solomon_decode(reed_solomon* rs, unsigned char **data_blocks, int block_size, unsigned char **dec_fec_blocks, unsigned int *fec_block_nos, unsigned int *erased_blocks, int nr_fec_blocks) {
    /* use stack instead of malloc, define a small number of DATA_SHARDS_MAX to save memory */
    gf dataDecodeMatrix[DATA_SHARDS_MAX*DATA_SHARDS_MAX];
    unsigned char* subShards[DATA_SHARDS_MAX];
    unsigned char* outputs[DATA_SHARDS_MAX];
    gf* m = rs->m;
    int i, j, c, swap, subMatrixRow, dataShards, nos, nshards;

    /* the erased_blocks should always sorted
     * if sorted, nr_fec_blocks times to check it
     * if not, sort it here
     * */
    for (i = 0; i < nr_fec_blocks; i++) {
        swap = 0;
        for (j = i+1; j < nr_fec_blocks; j++) {
            if (erased_blocks[i] > erased_blocks[j]) {
                /* the prefix is bigger than the following, swap */
                c = erased_blocks[i];
                erased_blocks[i] = erased_blocks[j];
                erased_blocks[j] = c;

                swap = 1;
            }
        }
        if (!swap)
            break;
    }

    j = 0;
    subMatrixRow = 0;
    nos = 0;
    nshards = 0;
    dataShards = rs->data_shards;
    for (i = 0; i < dataShards; i++) {
        if (j < nr_fec_blocks && i == erased_blocks[j])
            j++;
        else {
            /* this row is ok */
            for (c = 0; c < dataShards; c++)
                dataDecodeMatrix[subMatrixRow*dataShards + c] = m[i*dataShards + c];

            subShards[subMatrixRow] = data_blocks[i];
            subMatrixRow++;
        }
    }

    for (i = 0; i < nr_fec_blocks && subMatrixRow < dataShards; i++) {
        subShards[subMatrixRow] = dec_fec_blocks[i];
        j = dataShards + fec_block_nos[i];
        for (c = 0; c < dataShards; c++)
            dataDecodeMatrix[subMatrixRow*dataShards + c] = m[j*dataShards + c];

        subMatrixRow++;
    }

    if (subMatrixRow < dataShards)
        return -1;

    invert_mat(dataDecodeMatrix, dataShards);

    for (i = 0; i < nr_fec_blocks; i++) {
        j = erased_blocks[i];
        outputs[i] = data_blocks[j];
        memmove(dataDecodeMatrix+i*dataShards, dataDecodeMatrix+j*dataShards, dataShards);
    }

    return code_some_shards(dataDecodeMatrix, subShards, outputs, dataShards, nr_fec_blocks, block_size);
}

/**
 * encode a big size of buffer
 * input:
 * rs
 * nr_shards: assert(0 == nr_shards % rs->shards)
 * shards[nr_shards][block_size]
 * */
int reed_solomon_encode(reed_solomon* rs, unsigned char** shards, int nr_shards, int block_size) {
    unsigned char** data_blocks;
    unsigned char** fec_blocks;
    int i, ds = rs->data_shards, ps = rs->parity_shards, ss = rs->shards;
    i = nr_shards / ss;
    data_blocks = shards;
    fec_blocks = &shards[(i*ds)];

    for (i = 0; i < nr_shards; i += ss) {
        code_some_shards(rs->parity, data_blocks, fec_blocks, rs->data_shards, rs->parity_shards, block_size);
        data_blocks += ds;
        fec_blocks += ps;
    }
    return 0;
}

/**
 * reconstruct a big size of buffer
 * input:
 * rs
 * nr_shards: assert(0 == nr_shards % rs->data_shards)
 * shards[nr_shards][block_size]
 * marks[nr_shards] marks as errors
 * */
int reed_solomon_reconstruct(reed_solomon* rs, unsigned char** shards, unsigned char* marks, int nr_shards, int block_size) {
    unsigned char *dec_fec_blocks[DATA_SHARDS_MAX];
    unsigned int fec_block_nos[DATA_SHARDS_MAX];
    unsigned int erased_blocks[DATA_SHARDS_MAX];
    unsigned char* fec_marks;
    unsigned char **data_blocks, **fec_blocks;
    int i, j, dn, pn, n;
    int ds = rs->data_shards;
    int ps = rs->parity_shards;
    int err = 0;

    data_blocks = shards;
    n = nr_shards / rs->shards;
    fec_marks = marks + n*ds; //after all data, is't fec marks
    fec_blocks = shards + n*ds;

    for (j = 0; j < n; j++) {
        dn = 0;
        for (i = 0; i < ds; i++) {
            if (marks[i])
                erased_blocks[dn++] = i;
        }
        if (dn > 0) {
            pn = 0;
            for (i = 0; i < ps && pn < dn; i++) {
                if (!fec_marks[i]) {
                    //got valid fec row
                    fec_block_nos[pn] = i;
                    dec_fec_blocks[pn] = fec_blocks[i];
                    pn++;
                }
            }

            if (dn == pn) {
                reed_solomon_decode(rs, data_blocks, block_size, dec_fec_blocks, fec_block_nos, erased_blocks, dn);
            } else
                err = -1;
        }
        data_blocks += ds;
        marks += ds;
        fec_blocks += ps;
        fec_marks += ps;
    }

    return err;
}
