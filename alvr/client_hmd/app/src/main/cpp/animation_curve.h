#pragma once

#include <chrono>
#include <functional>
#include <cmath>
#include <algorithm>

// float value changing in time
class AnimationCurve {
public:
    // curve: function that maps 0 to 0, 1 to 1 and saturates to 1 for values > 1
    AnimationCurve(std::function<float(float)> curve, std::chrono::duration<float> duration);

    void Start(std::chrono::steady_clock::time_point instant, float startValue, float endValue);
    void Start(float startValue, float endValue) {
        Start(std::chrono::steady_clock::now(), startValue, endValue);
    }

    float GetValue(std::chrono::steady_clock::time_point instant);
    float GetValue() {
        return GetValue(std::chrono::steady_clock::now());
    }

private:
    std::function<float(float)> mCurve;
    std::chrono::duration<float> mDuration;
    std::chrono::steady_clock::time_point mStartInstant;
    float mStartValue, mEndValue;
};

inline float Linear(float input) {
    return std::min(input, 1.f);
}

inline float EaseOutSine(float input) {
    if (input < 1) {
        return std::sin(input * M_PI_2);
    } else {
        return 1;
    }
}