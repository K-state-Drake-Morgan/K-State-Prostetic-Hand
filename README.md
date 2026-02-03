# K-State-Prostetic-Hand
The code and tests for the Prostetic Hand

# Using Git

1. Clone the Repository:
  ```bash
  git clone https://github.com/K-state-Drake-Morgan/K-State-Prostetic-Hand.git
  ```
2. Move into the Repository:
  ```bash
  cd K-State-Prostetic-Hand
  ```
3. Create a Branch:
  ```bash
  git branch <feature name>
  ```
4. Move into the branch:
  ```bash
  git checkout <feature name>
  ```
5. Double check you branch with:
  ```bash
  git status
  ```
  It should look something like:
  ```bash
  On branch <feature name>
  nothing to commit, working tree clean
  ```
  or if you have changed files:
  ```bash
  On branch git_example
  Changes not staged for commit:
    (use "git add <file>..." to update what will be committed)
    (use "git restore <file>..." to discard changes in working directory)
          modified:   README.md
  
  no changes added to commit (use "git add" and/or "git commit -a")
  ```
6. Change files to your hearts content!
7. Save the current iteration of the file by committing them:
  ```bash
  git add <changed files, space seperated>
  git commit -m "<description of what you did>"
  ```
8. See that the commit has gone through with:
  ```bash
  git log
  ```
  Which should show something like:
  ```bash
  commit <hash> <Head status / Branch status>
  Author: <author> <<author email>>
  Date: <date of commit>
  
    <commit message>
    
  ```
  This part is unessisary but is a good thing to have in your toolbox when you need to find something.
9. Push to the server so everyone can see:
  ```bash
  git push origin -u
  ```
  1. You can have different origins named different things, but is sucks so we aren't doing that, you should be good by just using origin
  2. -u adds an upstream refrence to the branch, ie the origin will know that your local branch exists and makes it exist on the server
