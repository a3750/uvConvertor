#include <iostream>
#include "uvconvertor.hpp"
#include <filesystem>
#include <CLI/CLI.hpp>

using namespace std;
namespace fs=std::filesystem;

#define DEBUG 0

int main(int argc, char **argv)
{
	
	//CLI11
	CLI::App app("uVConvertor");
    // add version output
    app.set_version_flag("--version", std::string(CLI11_VERSION));
    std::vector<std::string> inputFiles;
    CLI::Option *opt = app.add_option("-f,--file", inputFiles, "uvProject File name")->check(CLI::ExistingFile)->required();

	std::string target;
    CLI::Option *topt = app.add_option("-t,--target", target, "uvProject target name");

	std::string outputFile;
    CLI::Option *copt = app.add_option("-o,--output",outputFile, "Output path")->check(CLI::ExistingDirectory);

	std::string extOptions;
	app.add_option("-e,--extoptions",extOptions,"External Options");
	
    CLI11_PARSE(app, argc, argv);

	// convertor to absolute path
	nlohmann::json json;
	for (auto inputFile : inputFiles) {
		fs::path in_path(inputFile);
		inputFile = std::filesystem::absolute(in_path).string();
	
	#if DEBUG
		cout<<"-------------------------------"<<endl;
		cout<<"input file:"<<inputFile<<endl;
		cout<<"output file:"<<outputFile<<endl;
		cout<<"ext options:"<<extOptions<<endl;
		cout<<"-------------------------------"<<endl;
	#endif
		uVConvertor uvc(inputFile, target);
		//uvc.printItems();
		auto j = uvc.toCompileJson(extOptions);
		for (auto item : j) {
			json += item;
		}
	}
	
	fs::path out_path(outputFile);
	if (std::ofstream fs(out_path / "compile_commands.json"); fs) {
		fs << std::setw(4) << json;
		cout << "Done." << endl;
	} else {
		cerr << "Cannot open file " << outputFile << endl;
	}
	
	return 0;
}
