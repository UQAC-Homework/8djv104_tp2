#ifndef SERVER_LOSSSIMULATOR_H
#define SERVER_LOSSSIMULATOR_H

#include <random>

class LossSimulator
{
public:
	explicit LossSimulator(const double probability) : drop(probability)
	{
	}

	bool should_drop() { return drop(rng); }

private:
	std::mt19937 rng{std::random_device{}()};
	std::bernoulli_distribution drop;
};

#endif //SERVER_LOSSSIMULATOR_H
