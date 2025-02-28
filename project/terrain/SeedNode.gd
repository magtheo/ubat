extends Node3D

signal noises_randomized

# Adjust to your actual resource paths:
const PATH_CORRAL   = "res://project/terrain/noise/coralNoise.tres"
const PATH_SAND     = "res://project/terrain/noise/sandNoise.tres"
const PATH_ROCK     = "res://project/terrain/noise/rockNoise.tres"
const PATH_KELP     = "res://project/terrain/noise/kelpNoise.tres"
const PATH_LAVAROCK = "res://project/terrain/noise/lavaRockNoise.tres"

const PATH_SECTION  = "res://project/terrain/noise/sectionNoise.tres"
const PATH_BLEND    = "res://project/terrain/noise/blendNoise.tres"

var corral_noise
var sand_noise
var rock_noise
var kelp_noise
var lavarock_noise
var section_noise
var blend_noise

#@export var seed = 12345

func _ready():
	corral_noise = load(PATH_CORRAL) as FastNoiseLite
	sand_noise = load(PATH_SAND) as FastNoiseLite
	rock_noise = load(PATH_ROCK) as FastNoiseLite
	kelp_noise = load(PATH_KELP) as FastNoiseLite
	lavarock_noise = load(PATH_LAVAROCK) as FastNoiseLite
	section_noise = load(PATH_SECTION) as FastNoiseLite
	blend_noise = load(PATH_BLEND) as FastNoiseLite
	
	print(corral_noise)
	randomize_noises() # TODO: seeds should later be randomized in the network node

func randomize_noises():
	var rng = RandomNumberGenerator.new()
	var new_seed = rng.randi()

	corral_noise.seed = new_seed
	sand_noise.seed = new_seed
	rock_noise.seed = new_seed
	kelp_noise.seed = new_seed
	lavarock_noise.seed = new_seed
	section_noise.seed = new_seed
	blend_noise.seed = new_seed
	print("Randomized noise seeds:", new_seed, "and applied to noises")
	emit_signal("noises_randomized")
