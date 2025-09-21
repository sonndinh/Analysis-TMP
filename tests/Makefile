ifndef LOKI_ROOT
	# Try if Loki library exists in the root of the project
	LOKI_ROOT = ../loki-0.1.7
endif

BUILD_DIR = build
SRC_DIR = ..

all: create_build_dir $(BUILD_DIR)/Zhang $(BUILD_DIR)/Deters

create_build_dir:
	@mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/Zhang: Zhang.cpp $(SRC_DIR)/Zhang_QPA_ECRTS09.h
	g++ -std=c++11 -I $(LOKI_ROOT)/include/ -I$(SRC_DIR) Zhang.cpp -o $(BUILD_DIR)/Zhang

$(BUILD_DIR)/Deters: Deters.cpp $(SRC_DIR)/Deters_RMAnalysis_RTAS03.h
	g++ -std=c++11 -I $(LOKI_ROOT)/include/ -I$(SRC_DIR) Deters.cpp -o $(BUILD_DIR)/Deters

clean:
	rm -rf $(BUILD_DIR)
