#include "display.hpp"

wsi::display &wsi::display::get() {
    static display instance;
    return instance;
}
