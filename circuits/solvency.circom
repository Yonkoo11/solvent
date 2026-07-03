pragma circom 2.0.0;

include "bitify.circom"; // Num2Bits — bit decomposition, field-agnostic

// Solvency / Proof-of-Reserves circuit.
//
// The issuer proves, WITHOUT revealing any individual customer balance, that:
//   1. every balance is a non-negative amount that fits in 64 bits
//      (this kills the Mt.Gox trick of inserting a "negative" liability —
//       encoded as a field element near p — to shrink the reported total)
//   2. the sum of all balances equals the publicly-published `total`.
//
// The single public signal is `total`. The on-chain Soroban contract binds
// `total` as the Fr public input and then checks reserve >= total.
template Solvency(N) {
    signal input balances[N]; // private: per-customer liabilities
    signal input total;       // public:  claimed sum of liabilities

    // Range-check every balance to [0, 2^64).
    component rc[N];

    // Running-sum accumulator.
    signal sumAcc[N + 1];
    sumAcc[0] <== 0;

    for (var i = 0; i < N; i++) {
        rc[i] = Num2Bits(64);
        rc[i].in <== balances[i];
        sumAcc[i + 1] <== sumAcc[i] + balances[i];
    }

    // Enforce the published total equals the true sum of liabilities.
    total === sumAcc[N];
}

component main { public [total] } = Solvency(8);
