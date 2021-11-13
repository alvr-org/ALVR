#pragma once

#include "shared/threadtools.h"

class CEncoder : public CThread {
  public:
    CEncoder() {}
    ~CEncoder() {}
    bool Init() override { return true; }
    void Run() override {}

    void Stop() {}
    void OnPacketLoss() {}
    void InsertIDR() {}
};
