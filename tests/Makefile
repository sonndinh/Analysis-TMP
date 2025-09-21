BUILD_DIR = build
SRC_DIR = ..
LOKI_DIR = ../loki-0.1.7

all: create_build_dir $(BUILD_DIR)/Zhang $(BUILD_DIR)/Deters

create_build_dir:
	@mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/Zhang: Zhang.cpp $(SRC_DIR)/Zhang_QPA_ECRTS09.h
	g++ -std=c++11 -I $(LOKI_DIR)/include/ -I$(SRC_DIR) Zhang.cpp -o $(BUILD_DIR)/Zhang

$(BUILD_DIR)/Deters: Deters.cpp $(SRC_DIR)/Deters_RMAnalysis_RTAS03.h
	g++ -std=c++11 -I $(LOKI_DIR)/include/ -I$(SRC_DIR) Deters.cpp -o $(BUILD_DIR)/Deters
